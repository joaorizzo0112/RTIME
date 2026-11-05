use std::ptr::null_mut;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE};
use windows_sys::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives,
};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::SystemInformation::{
    GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX,
};
use windows_sys::Win32::System::Threading::{
    GetSystemTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;

use super::wide;

/// Processo consultado pela API nativa do Windows. A classificação usa o  working set (memória física mapeada), e não estimativas de uso de CPU.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct ProcessEntry {
    pub name: [u16; 32],
    pub working_set_bytes: u64,
}
impl ProcessEntry {
    pub fn display_name(&self) -> String {
        let end = self
            .name
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.name.len());
        String::from_utf16_lossy(&self.name[..end])
    }
}

// Top 3 ordenado com a maior quantidade de RAM primeiro; evita ordenar
// listas grandes e não mantém handles nem dados pessoais em disco.
fn insert_top_process(top: &mut [ProcessEntry; 3], item: ProcessEntry) {
    if item.working_set_bytes == 0 {
        return;
    }
    if let Some(at) = top
        .iter()
        .position(|p| item.working_set_bytes > p.working_set_bytes)
    {
        for j in (at + 1..top.len()).rev() {
            top[j] = top[j - 1];
        }
        top[at] = item;
    }
}

#[derive(Clone, Copy, Default)]
pub struct Metrics {
    pub cpu_pct: f64,
    pub ram_pct: f64,
    pub ram_used_gib: f64,
    pub ram_total_gib: f64,
    pub disk_pct: f64,
    pub disk_free_gib: f64,
    pub disk_total_gib: f64,
    pub disk_available: bool,
    pub down_mib: f64,
    pub up_mib: f64,
    /// Contadores acumulados pela interface, nao consumo mensal ou medido desde que o app abriu.
    pub network_total_bytes: u64,
    pub network_available: bool,
    pub uptime_ms: u64,
    pub process_count: u32,
    pub top_processes: [ProcessEntry; 3],
}

pub struct Sampler {
    previous_cpu: Option<(u64, u64)>,
    previous_network: Option<(u64, u64, Instant)>,
    previous: Metrics,
    last_disk: Option<Instant>,
    last_processes: Option<Instant>,
    last_disk_selection: u32,
    last_adapter_selection: u32,
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            previous_cpu: None,
            previous_network: None,
            previous: Metrics::default(),
            last_disk: None,
            last_processes: None,
            last_disk_selection: u32::MAX,
            last_adapter_selection: u32::MAX,
        }
    }

    pub fn reset_network_baseline(&mut self) {
        self.previous_network = None;
    }

    pub fn read(&mut self, disk_selection: u32, adapter_selection: u32) -> Metrics {
        if self.last_disk_selection != disk_selection {
            self.last_disk_selection = disk_selection;
            self.last_disk = None;
            self.previous.disk_pct = 0.0;
            self.previous.disk_free_gib = 0.0;
            self.previous.disk_total_gib = 0.0;
            self.previous.disk_available = false;
        }
        if self.last_adapter_selection != adapter_selection {
            self.last_adapter_selection = adapter_selection;
            self.previous_network = None;
            self.previous.down_mib = 0.0;
            self.previous.up_mib = 0.0;
            self.previous.network_total_bytes = 0;
            self.previous.network_available = false;
        }
        let mut result = self.previous;
        let now = Instant::now();
        unsafe {
            let mut idle: FILETIME = std::mem::zeroed();
            let mut kernel: FILETIME = std::mem::zeroed();
            let mut user: FILETIME = std::mem::zeroed();
            if GetSystemTimes(&mut idle, &mut kernel, &mut user) != 0 {
                let now_idle = filetime_u64(idle);
                let now_total = filetime_u64(kernel).saturating_add(filetime_u64(user));
                if let Some((old_idle, old_total)) = self.previous_cpu {
                    let dt = now_total.saturating_sub(old_total);
                    if dt > 0 {
                        let busy = dt.saturating_sub(now_idle.saturating_sub(old_idle));
                        result.cpu_pct = clamp_pct(100.0 * busy as f64 / dt as f64);
                    }
                }
                self.previous_cpu = Some((now_idle, now_total));
            }

            let mut mem: MEMORYSTATUSEX = std::mem::zeroed();
            mem.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut mem) != 0 {
                result.ram_pct = mem.dwMemoryLoad as f64;
                result.ram_total_gib = mem.ullTotalPhys as f64 / 1073741824.0;
                result.ram_used_gib =
                    mem.ullTotalPhys.saturating_sub(mem.ullAvailPhys) as f64 / 1073741824.0;
            }

            if self.last_disk.map_or(true, |last| {
                now.duration_since(last) >= Duration::from_secs(10)
            }) {
                self.last_disk = Some(now);
                let root = disk_root(disk_selection);
                let mut available = 0u64;
                let mut total = 0u64;
                let mut free = 0u64;
                result.disk_available = false;
                result.disk_pct = 0.0;
                result.disk_free_gib = 0.0;
                result.disk_total_gib = 0.0;
                if GetDiskFreeSpaceExW(wide(&root).as_ptr(), &mut available, &mut total, &mut free)
                    != 0
                    && total > 0
                {
                    result.disk_available = true;
                    result.disk_pct = clamp_pct(100.0 * (1.0 - free as f64 / total as f64));
                    result.disk_free_gib = free as f64 / 1073741824.0;
                    result.disk_total_gib = total as f64 / 1073741824.0;
                }
            }

            let mut table: *mut MIB_IF_TABLE2 = null_mut();
            if GetIfTable2(&mut table) == 0 && !table.is_null() {
                let mut down = 0u64;
                let mut up = 0u64;
                let len = ((*table).NumEntries as usize).min(4096);
                let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), len);
                let mut matched = false;
                for interface in rows {
                    if interface.OperStatus == 1
                        && interface.Type != 24
                        && (adapter_selection == 0 || interface.InterfaceIndex == adapter_selection)
                    {
                        matched = true;
                        down = down.saturating_add(interface.InOctets);
                        up = up.saturating_add(interface.OutOctets);
                    }
                }
                let sample_at = Instant::now();
                result.network_available = matched;
                result.down_mib = 0.0;
                result.up_mib = 0.0;
                result.network_total_bytes = 0;
                if matched {
                    result.network_total_bytes = down.saturating_add(up);
                    if let Some((prev_down, prev_up, prev_time)) = self.previous_network {
                        let dt = sample_at.duration_since(prev_time).as_secs_f64();
                        if dt >= 0.1 && down >= prev_down && up >= prev_up {
                            result.down_mib = (down - prev_down) as f64 / dt / (1024.0 * 1024.0);
                            result.up_mib = (up - prev_up) as f64 / dt / (1024.0 * 1024.0);
                        }
                    }
                    self.previous_network = Some((down, up, sample_at));
                } else {
                    self.previous_network = None;
                }
                FreeMibTable(table.cast());
            } else {
                result.network_available = false;
                result.down_mib = 0.0;
                result.up_mib = 0.0;
                result.network_total_bytes = 0;
                self.previous_network = None;
            }
            result.uptime_ms = GetTickCount64();

            if self.last_processes.map_or(true, |last| {
                now.duration_since(last) >= Duration::from_secs(5)
            }) {
                self.last_processes = Some(now);
                let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
                if snapshot != INVALID_HANDLE_VALUE {
                    let mut entry: PROCESSENTRY32W = std::mem::zeroed();
                    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
                    let mut count = 0u32;
                    let mut top = [ProcessEntry::default(); 3];
                    if Process32FirstW(snapshot, &mut entry) != 0 {
                        loop {
                            count = count.saturating_add(1);
                            let handle = OpenProcess(
                                PROCESS_QUERY_LIMITED_INFORMATION,
                                0,
                                entry.th32ProcessID,
                            );
                            if !handle.is_null() {
                                let mut memory: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
                                let memory_size =
                                    std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                                memory.cb = memory_size;
                                if K32GetProcessMemoryInfo(handle, &mut memory, memory_size) != 0
                                    && memory.WorkingSetSize > 0
                                {
                                    let mut process = ProcessEntry::default();
                                    let end = entry
                                        .szExeFile
                                        .iter()
                                        .position(|&c| c == 0)
                                        .unwrap_or(entry.szExeFile.len());
                                    let n = end.min(process.name.len() - 1);
                                    process.name[..n].copy_from_slice(&entry.szExeFile[..n]);
                                    process.working_set_bytes = memory.WorkingSetSize as u64;
                                    insert_top_process(&mut top, process);
                                }
                                CloseHandle(handle);
                            }
                            if Process32NextW(snapshot, &mut entry) == 0 {
                                break;
                            }
                        }
                    }
                    result.process_count = count;
                    result.top_processes = top;
                    CloseHandle(snapshot);
                }
            }
        }
        self.previous = result;
        result
    }
}

pub fn disk_root(selection: u32) -> String {
    if let Some(letter) = letter_for_disk(selection) {
        format!("{}:\\", letter)
    } else {
        let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        format!("{}\\", drive.trim_end_matches('\\'))
    }
}

pub fn letter_for_disk(selection: u32) -> Option<char> {
    (1..=26)
        .contains(&selection)
        .then(|| char::from(b'A' + (selection - 1) as u8))
}

pub fn disk_label(selection: u32) -> String {
    if let Some(letter) = letter_for_disk(selection) {
        format!("{}:", letter)
    } else {
        std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into())
    }
}

pub fn available_drives() -> Vec<(u32, String)> {
    let mut output = Vec::new();
    unsafe {
        let mask = GetLogicalDrives();
        for index in 0..26u32 {
            if mask & (1u32 << index) == 0 {
                continue;
            }
            let selection = index + 1;
            let root = disk_root(selection);
            let kind = GetDriveTypeW(wide(&root).as_ptr());
            if kind == DRIVE_FIXED || kind == DRIVE_REMOVABLE {
                output.push((
                    selection,
                    format!(
                        "{}  -  {}",
                        disk_label(selection),
                        if kind == DRIVE_FIXED {
                            "Unidade local"
                        } else {
                            "Removivel"
                        }
                    ),
                ));
            }
        }
    }
    output
}

pub fn available_adapters() -> Vec<(u32, String)> {
    let mut output = Vec::new();
    unsafe {
        let mut table: *mut MIB_IF_TABLE2 = null_mut();
        if GetIfTable2(&mut table) != 0 || table.is_null() {
            return output;
        }
        let rows = std::slice::from_raw_parts(
            (*table).Table.as_ptr(),
            ((*table).NumEntries as usize).min(4096),
        );
        for row in rows {
            if row.OperStatus != 1 || row.Type == 24 {
                continue;
            }
            let end = row
                .Alias
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(row.Alias.len());
            let alias = String::from_utf16_lossy(&row.Alias[..end]);
            let name = if alias.is_empty() {
                format!("Interface {}", row.InterfaceIndex)
            } else {
                alias
            };
            output.push((
                row.InterfaceIndex,
                format!("{} (#{})", name, row.InterfaceIndex),
            ));
        }
        FreeMibTable(table.cast());
    }
    output.sort_by(|a, b| a.1.cmp(&b.1));
    output.dedup_by_key(|entry| entry.0);
    output
}

fn filetime_u64(ft: FILETIME) -> u64 {
    (ft.dwHighDateTime as u64) << 32 | ft.dwLowDateTime as u64
}

pub fn clamp_pct(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        0.0
    }
}

pub fn fmt_uptime(ms: u64) -> String {
    let minutes = ms / 60_000;
    let hours = minutes / 60;
    if hours >= 24 {
        format!("{}d {}h", hours / 24, hours % 24)
    } else {
        format!("{}h {}m", hours, minutes % 60)
    }
}

pub fn fmt_network(value_mib: f64) -> String {
    if value_mib < 1.0 {
        format!("{:.0} KiB/s", value_mib.max(0.0) * 1024.0)
    } else {
        format!("{value_mib:.2} MiB/s")
    }
}

pub fn fmt_network_total(bytes: u64) -> String {
    if bytes == 0 {
        return "0 KiB".to_string();
    }
    let amount = bytes as f64;
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    const TIB: f64 = GIB * 1024.0;
    const PIB: f64 = TIB * 1024.0;
    if amount >= PIB {
        format!("{:.2} PiB", amount / PIB)
    } else if amount >= TIB {
        format!("{:.2} TiB", amount / TIB)
    } else if amount >= GIB {
        format!("{:.2} GiB", amount / GIB)
    } else if amount >= MIB {
        format!("{:.2} MiB", amount / MIB)
    } else {
        format!("{:.1} KiB", amount / KIB)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uptime() {
        assert_eq!(fmt_uptime(3_840_000), "1h 4m");
    }
    #[test]
    fn uptime_days() {
        assert_eq!(fmt_uptime(90_000_000), "1d 1h");
    }
    #[test]
    fn pct_bounds() {
        assert_eq!(clamp_pct(125.0), 100.0);
        assert_eq!(clamp_pct(-7.0), 0.0);
    }
    #[test]
    fn network_units() {
        assert_eq!(fmt_network(0.5), "512 KiB/s");
    }
    #[test]
    fn network_total_auto_units() {
        assert_eq!(fmt_network_total(0), "0 KiB");
        assert_eq!(fmt_network_total(1536), "1.5 KiB");
        assert_eq!(fmt_network_total(1024 * 1024), "1.00 MiB");
        assert_eq!(fmt_network_total(15 * 1024_u64.pow(3)), "15.00 GiB");
        assert_eq!(fmt_network_total(2 * 1024_u64.pow(4)), "2.00 TiB");
        assert!(fmt_network_total(u64::MAX).ends_with(" PiB"));
    }
}

#[cfg(test)]
mod sources_v038_tests {
    use super::*;
    #[test]
    fn drive_letter_mapping() {
        assert_eq!(letter_for_disk(0), None);
        assert_eq!(letter_for_disk(1), Some('A'));
        assert_eq!(letter_for_disk(3), Some('C'));
        assert_eq!(letter_for_disk(26), Some('Z'));
        assert_eq!(letter_for_disk(27), None);
    }
    #[test]
    fn manual_drive_root() {
        assert_eq!(disk_root(4), "D:\\");
    }
}

#[cfg(test)]
mod process_tests {
    use super::*;
    #[test]
    fn biggest_working_sets_are_sorted_and_limited() {
        let mut top = [ProcessEntry::default(); 3];
        for mb in [22u64, 15, 60, 35, 0, 20] {
            let mut item = ProcessEntry::default();
            item.working_set_bytes = mb * 1024 * 1024;
            insert_top_process(&mut top, item);
        }
        assert_eq!(
            top.map(|p| p.working_set_bytes / (1024 * 1024)),
            [60, 35, 22]
        );
    }
    #[test]
    fn name_decodes_utf16_until_null() {
        let mut p = ProcessEntry::default();
        p.name[..3].copy_from_slice(&['a' as u16, 'b' as u16, 'c' as u16]);
        assert_eq!(p.display_name(), "abc");
    }
}
