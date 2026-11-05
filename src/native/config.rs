//! Preferências Win32 persistidas no Registro do usuário.
use super::wide;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_DWORD,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN};

pub const N: usize = 6;
pub const INTERFACE_THEMES: [&str; 2] = ["Claro", "Escuro"];
pub const THEMES: [&str; 3] = ["Escuro", "Claro", "Automático (Windows)"];
pub const STYLES: [&str; 3] = ["Fluent Light", "Midnight Minimal", "Compact Dashboard"];
pub const ACCENTS: [&str; 6] = [
    "Padrão do estilo",
    "Azul",
    "Violeta",
    "Esmeralda",
    "Âmbar",
    "Coral",
];
pub const SIZES: [&str; 4] = ["Compacto", "Padrão", "Expandido", "Horizontal"];
pub const OPACITIES: [&str; 3] = ["100%", "94%", "86%"];
// A opcao zero preserva o valor definido para todos os widgets.
pub const INDIVIDUAL_STYLES: [&str; 4] = [
    "Seguir estilo global",
    "Fluent Light",
    "Midnight Minimal",
    "Compact Dashboard",
];
pub const INDIVIDUAL_ACCENTS: [&str; 7] = [
    "Seguir cor global",
    "Padrao do estilo",
    "Azul",
    "Violeta",
    "Esmeralda",
    "Ambar",
    "Coral",
];
pub const GRAPH_MODES: [&str; 5] = [
    "Seguir configuracao global",
    "Linha",
    "Area suave",
    "Barras verticais",
    "Desligado",
];
pub const FONT_SCALES: [&str; 3] = ["Padrao", "Numeros maiores", "Numeros discretos"];
pub const BORDER_MODES: [&str; 3] = ["Sutil", "Cor de destaque", "Sem borda"];
pub const INDIVIDUAL_OPACITIES: [&str; 4] = ["Seguir opacidade global", "100%", "94%", "86%"];
pub const VISIBILITY_CHOICES: [&str; 3] =
    ["Seguir configuracao global", "Sempre exibir", "Ocultar"];
pub const UPDATE_INTERVALS: [&str; 4] = ["0,5 segundo", "1 segundo", "2 segundos", "5 segundos"];
pub const LAYOUT_MODES: [&str; 3] = ["Grade", "Linhas", "Colunas"];
pub const LAYOUT_GAPS: [&str; 4] = [
    "8 px · justo",
    "14 px · confortável",
    "22 px · amplo",
    "32 px · espaçado",
];

pub fn update_interval_ms(selection: u32) -> u32 {
    match selection {
        0 => 500,
        2 => 2000,
        3 => 5000,
        _ => 1000,
    }
}

const KEY: &str = "Software\\Rtime";
const LEGACY_KEY: &str = "Software\\RustFetchNative";

// A chave nova tem prioridade; a legada é consultada uma única vez na migração.
fn registry_read_keys() -> [&'static str; 2] {
    [KEY, LEGACY_KEY]
}

const WINDOWS_THEME_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize";

pub fn dimensions(size: u32) -> (i32, i32) {
    match size {
        0 => (164, 128),
        2 => (232, 182),
        3 => (440, 140),
        _ => (180, 150),
    }
}

pub fn positions_for_sizes(sizes: &[u32; N]) -> [(i32, i32); N] {
    let first_col_width = (0..N)
        .step_by(2)
        .map(|i| dimensions(sizes[i]).0)
        .max()
        .unwrap_or(180);
    let second_col_width = (1..N)
        .step_by(2)
        .map(|i| dimensions(sizes[i]).0)
        .max()
        .unwrap_or(180);
    let total_width = first_col_width + second_col_width + 14;
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let x0 = (screen_width - total_width - 32).max(24);
    let row_heights: [i32; 3] = std::array::from_fn(|r| {
        dimensions(sizes[r * 2])
            .1
            .max(dimensions(sizes[r * 2 + 1]).1)
    });
    std::array::from_fn(|i| {
        let row = i / 2;
        let x = x0 + if i % 2 == 0 { 0 } else { first_col_width + 14 };
        let y = 60 + row_heights[..row].iter().sum::<i32>() + row as i32 * 14;
        (x, y)
    })
}

pub fn positions_for(size: u32) -> [(i32, i32); N] {
    positions_for_sizes(&[size; N])
}

pub fn legacy_positions_for(size: u32) -> [(i32, i32); N] {
    let (width, height) = dimensions(size);
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let x0 = (screen_width - width * 2 - 54).max(24);
    std::array::from_fn(|i| {
        (
            x0 + (i as i32 % 2) * (width + 14),
            60 + (i as i32 / 2) * (height + 14),
        )
    })
}

#[derive(Clone, Debug)]
pub struct Config {
    pub visible: bool,
    pub locked: bool,
    pub behind: bool,
    pub snap_enabled: bool,
    pub group_move: bool,
    pub layout_mode: u32,
    pub layout_anchor: u32,
    pub layout_monitor: u32,
    pub layout_gap: u32,
    pub enabled: [bool; N],
    pub positions: [(i32, i32); N],
    pub theme: u32,
    pub interface_theme: u32,
    pub style: u32,
    pub accent: u32,
    pub size: u32,
    pub card_sizes: [u32; N],
    pub opacity: u32,
    pub show_graphs: bool,
    pub show_bars: bool,
    pub show_subtitles: bool,
    pub widget_styles: [u32; N],
    pub widget_accents: [u32; N],
    pub widget_graphs: [u32; N],
    pub widget_fonts: [u32; N],
    pub widget_borders: [u32; N],
    pub widget_opacities: [u32; N],
    pub widget_bars: [u32; N],
    pub widget_subtitles: [u32; N],
    pub update_interval: u32,
    pub monitor_disk: u32,
    pub monitor_adapter: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            visible: true,
            locked: false,
            behind: true,
            snap_enabled: true,
            group_move: false,
            layout_mode: 0,
            layout_anchor: 0,
            layout_monitor: 0,
            layout_gap: 1,
            enabled: [true; N],
            positions: positions_for(1),
            theme: 0,
            interface_theme: 0,
            style: 1, 
            accent: 0,
            size: 1,
            card_sizes: [1; N],
            opacity: 0,
            show_graphs: true,
            show_bars: true,
            show_subtitles: true,
            widget_styles: [0; N],
            widget_accents: [0; N],
            widget_graphs: [0; N],
            widget_fonts: [0; N],
            widget_borders: [0; N],
            widget_opacities: [0; N],
            widget_bars: [0; N],
            widget_subtitles: [0; N],
            update_interval: 1,
            monitor_disk: 0,
            monitor_adapter: 0,
        }
    }
}

unsafe fn dword(key: HKEY, name: &str, fallback: u32) -> u32 {
    let mut value = fallback;
    let mut kind = 0u32;
    let mut size = 4u32;
    let result = RegQueryValueExW(
        key,
        wide(name).as_ptr(),
        null(),
        &mut kind,
        (&mut value as *mut u32).cast(),
        &mut size,
    );
    if result == ERROR_SUCCESS && kind == REG_DWORD && size == 4 {
        value
    } else {
        fallback
    }
}

unsafe fn write(key: HKEY, name: &str, value: u32) {
    let _ = RegSetValueExW(
        key,
        wide(name).as_ptr(),
        0,
        REG_DWORD,
        (&value as *const u32).cast(),
        4,
    );
}

pub fn system_light_theme() -> bool {
    unsafe {
        let mut key: HKEY = null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            wide(WINDOWS_THEME_KEY).as_ptr(),
            0,
            KEY_READ,
            &mut key,
        ) != ERROR_SUCCESS
        {
            return false;
        }
        let result = dword(key, "AppsUseLightTheme", 0) != 0;
        RegCloseKey(key);
        result
    }
}

impl Config {
    pub fn copy_widget_visual_to_all(&mut self, source: usize) {
        if source >= N {
            return;
        }
        for target in 0..N {
            if target == source {
                continue;
            }
            self.widget_styles[target] = self.widget_styles[source];
            self.widget_accents[target] = self.widget_accents[source];
            self.widget_graphs[target] = self.widget_graphs[source];
            self.widget_fonts[target] = self.widget_fonts[source];
            self.widget_borders[target] = self.widget_borders[source];
            self.widget_opacities[target] = self.widget_opacities[source];
            self.widget_bars[target] = self.widget_bars[source];
            self.widget_subtitles[target] = self.widget_subtitles[source];
        }
    }

    pub fn reset_widget_visual(&mut self, index: usize) {
        if index >= N {
            return;
        }
        self.widget_styles[index] = 0;
        self.widget_accents[index] = 0;
        self.widget_graphs[index] = 0;
        self.widget_fonts[index] = 0;
        self.widget_borders[index] = 0;
        self.widget_opacities[index] = 0;
        self.widget_bars[index] = 0;
        self.widget_subtitles[index] = 0;
    }

    pub fn load() -> Self {
        let mut cfg = Self::default();
        unsafe {
            let mut key: HKEY = null_mut();
            let mut migrated_from_legacy = false;
            let mut found = false;

            for path in registry_read_keys() {
                if RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    wide(path).as_ptr(),
                    0,
                    KEY_READ,
                    &mut key,
                ) == ERROR_SUCCESS
                {
                    migrated_from_legacy = path == LEGACY_KEY;
                    found = true;
                    break;
                }
            }
            if !found {
                return cfg;
            }
            cfg.visible = dword(key, "Visible", 1) != 0;
            cfg.locked = dword(key, "Locked", 0) != 0;
            cfg.behind = dword(key, "AlwaysBehind", 1) != 0;
            cfg.snap_enabled = dword(key, "SnapEnabled", 1) != 0;
            cfg.group_move = dword(key, "GroupMove", 0) != 0;
            cfg.layout_mode = dword(key, "LayoutMode", 0).min(2);
            cfg.layout_anchor = dword(key, "LayoutAnchor", 0).min(8);
            cfg.layout_monitor = dword(key, "LayoutMonitor", 0).min(16);
            cfg.layout_gap = dword(key, "LayoutGap", 1).min(3);
            cfg.theme = dword(key, "Theme", 0).min(2);
            cfg.interface_theme = dword(key, "InterfaceTheme", 0).min(1);
            let prior_style = dword(key, "VisualStyle", u32::MAX);
            cfg.style = if prior_style == u32::MAX {
                1
            } else {
                prior_style.min(2)
            };
            let saved_accent = dword(key, "Accent", u32::MAX);
            cfg.accent = if saved_accent == u32::MAX {
                0
            } else if prior_style == u32::MAX {
                saved_accent.min(4) + 1
            } else {
                saved_accent.min(5)
            };
            cfg.size = dword(key, "CardSize", 1).min(3);
            cfg.opacity = dword(key, "Opacity", 0).min(2);
            cfg.show_graphs = dword(key, "ShowGraphs", 1) != 0;
            cfg.show_bars = dword(key, "ShowBars", 1) != 0;
            cfg.show_subtitles = dword(key, "ShowSubtitles", 1) != 0;
            cfg.update_interval = dword(key, "UpdateInterval", 1).min(3);
            cfg.monitor_disk = dword(key, "MonitorDisk", 0).min(26);
            cfg.monitor_adapter = dword(key, "MonitorAdapterIndex", 0);
            for i in 0..N {
                cfg.card_sizes[i] = dword(key, &format!("CardSize{i}"), cfg.size).min(3);
                cfg.enabled[i] = dword(key, &format!("Enabled{i}"), 1) != 0;
                cfg.widget_styles[i] = dword(key, &format!("WidgetStyle{i}"), 0).min(3);
                cfg.widget_accents[i] = dword(key, &format!("WidgetAccent{i}"), 0).min(6);
                cfg.widget_graphs[i] = dword(key, &format!("WidgetGraph{i}"), 0).min(4);
                cfg.widget_fonts[i] = dword(key, &format!("WidgetFont{i}"), 0).min(2);
                cfg.widget_borders[i] = dword(key, &format!("WidgetBorder{i}"), 0).min(2);
                cfg.widget_opacities[i] = dword(key, &format!("WidgetOpacity{i}"), 0).min(3);
                cfg.widget_bars[i] = dword(key, &format!("WidgetBars{i}"), 0).min(2);
                cfg.widget_subtitles[i] = dword(key, &format!("WidgetSubtitles{i}"), 0).min(2);
            }
            let default_positions = legacy_positions_for(cfg.size); // Compatibilidade com v0.3.0.
            for i in 0..N {
                let (dx, dy) = default_positions[i];
                let x = dword(key, &format!("X{i}"), dx as u32) as i32;
                let y = dword(key, &format!("Y{i}"), dy as u32) as i32;
                if x.abs() < 30_000 && y.abs() < 30_000 {
                    cfg.positions[i] = (x, y);
                }
            }
            RegCloseKey(key);

            if migrated_from_legacy {
                cfg.save();
            }
        }
        cfg
    }

    pub fn save(&self) {
        unsafe {
            let mut key: HKEY = null_mut();
            if RegCreateKeyExW(
                HKEY_CURRENT_USER,
                wide(KEY).as_ptr(),
                0,
                null_mut(),
                0,
                KEY_SET_VALUE,
                null(),
                &mut key,
                null_mut(),
            ) != ERROR_SUCCESS
            {
                return;
            }
            write(key, "Visible", self.visible as u32);
            write(key, "Locked", self.locked as u32);
            write(key, "AlwaysBehind", self.behind as u32);
            write(key, "SnapEnabled", self.snap_enabled as u32);
            write(key, "GroupMove", self.group_move as u32);
            write(key, "LayoutMode", self.layout_mode);
            write(key, "LayoutAnchor", self.layout_anchor);
            write(key, "LayoutMonitor", self.layout_monitor);
            write(key, "LayoutGap", self.layout_gap);
            write(key, "Theme", self.theme);
            write(key, "InterfaceTheme", self.interface_theme.min(1));
            write(key, "VisualStyle", self.style);
            write(key, "Accent", self.accent);
            write(key, "CardSize", self.size);
            write(key, "Opacity", self.opacity);
            write(key, "ShowGraphs", self.show_graphs as u32);
            write(key, "ShowBars", self.show_bars as u32);
            write(key, "ShowSubtitles", self.show_subtitles as u32);
            write(key, "UpdateInterval", self.update_interval);
            write(key, "MonitorDisk", self.monitor_disk.min(26));
            write(key, "MonitorAdapterIndex", self.monitor_adapter);
            for i in 0..N {
                write(key, &format!("CardSize{i}"), self.card_sizes[i]);
                write(key, &format!("Enabled{i}"), self.enabled[i] as u32);
                write(key, &format!("WidgetStyle{i}"), self.widget_styles[i]);
                write(key, &format!("WidgetAccent{i}"), self.widget_accents[i]);
                write(key, &format!("WidgetGraph{i}"), self.widget_graphs[i]);
                write(key, &format!("WidgetFont{i}"), self.widget_fonts[i]);
                write(key, &format!("WidgetBorder{i}"), self.widget_borders[i]);
                write(key, &format!("WidgetOpacity{i}"), self.widget_opacities[i]);
                write(key, &format!("WidgetBars{i}"), self.widget_bars[i]);
                write(
                    key,
                    &format!("WidgetSubtitles{i}"),
                    self.widget_subtitles[i],
                );
                write(key, &format!("X{i}"), self.positions[i].0 as u32);
                write(key, &format!("Y{i}"), self.positions[i].1 as u32);
            }
            RegCloseKey(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dimension_presets() {
        assert_eq!(dimensions(1), (180, 150));
        assert_eq!(dimensions(3), (440, 140));
        assert_eq!(SIZES.len(), 4);
    }
    #[test]
    fn magnetic_snap_is_on_by_default() {
        assert!(Config::default().snap_enabled);
    }
    #[test]
    fn default_is_midnight() {
        let cfg = Config::default();
        assert_eq!(cfg.style, 1);
        assert!(cfg.show_graphs && cfg.show_subtitles);
    }
    #[test]
    fn mixed_sizes_do_not_overlap_in_default_columns() {
        let sizes = [3, 0, 2, 1, 0, 3];
        let positions = positions_for_sizes(&sizes);
        for (i, (x, y)) in positions.iter().enumerate() {
            for j in i + 1..N {
                let (xj, yj) = positions[j];
                let (w, h) = dimensions(sizes[i]);
                let (wj, hj) = dimensions(sizes[j]);
                assert!(*x + w <= xj || xj + wj <= *x || *y + h <= yj || yj + hj <= *y);
            }
        }
    }
}

#[cfg(test)]
mod personalization_tests {
    use super::*;

    #[test]
    fn update_rates_are_bounded() {
        assert_eq!(update_interval_ms(0), 500);
        assert_eq!(update_interval_ms(1), 1000);
        assert_eq!(update_interval_ms(2), 2000);
        assert_eq!(update_interval_ms(3), 5000);
    }

    #[test]
    fn widget_visual_defaults_inherit_global_settings() {
        let mut cfg = Config::default();
        assert_eq!(cfg.widget_styles, [0; N]);
        assert_eq!(cfg.widget_graphs, [0; N]);
        cfg.widget_styles[2] = 3;
        cfg.widget_graphs[2] = 2;
        cfg.widget_fonts[2] = 1;
        cfg.reset_widget_visual(2);
        assert_eq!(cfg.widget_styles[2], 0);
        assert_eq!(cfg.widget_graphs[2], 0);
        assert_eq!(cfg.widget_fonts[2], 0);
    }
}

#[cfg(test)]
mod layout_prefs_tests {
    use super::*;
    #[test]
    fn new_layout_options_do_not_change_previous_defaults() {
        let cfg = Config::default();
        assert!(!cfg.group_move);
        assert_eq!(cfg.layout_mode, 0);
        assert_eq!(cfg.layout_anchor, 0);
        assert_eq!(cfg.layout_monitor, 0);
        assert_eq!(cfg.layout_gap, 1);
        assert_eq!(cfg.widget_styles.len(), N);
    }
}

#[cfg(test)]
mod experience_v037_tests {
    use super::*;

    #[test]
    fn copy_visual_ignores_positions_sizes_and_enabled() {
        let mut cfg = Config::default();
        cfg.widget_styles[2] = 3;
        cfg.widget_accents[2] = 4;
        cfg.widget_graphs[2] = 3;
        cfg.widget_fonts[2] = 1;
        cfg.widget_borders[2] = 1;
        cfg.widget_opacities[2] = 2;
        cfg.widget_bars[2] = 2;
        cfg.widget_subtitles[2] = 1;
        cfg.card_sizes = [0, 1, 2, 3, 0, 1];
        cfg.enabled[4] = false;
        let positions = cfg.positions;
        let sizes = cfg.card_sizes;
        let enabled = cfg.enabled;
        cfg.copy_widget_visual_to_all(2);
        assert_eq!(cfg.widget_styles, [3; N]);
        assert_eq!(cfg.widget_accents, [4; N]);
        assert_eq!(cfg.widget_graphs, [3; N]);
        assert_eq!(cfg.widget_fonts, [1; N]);
        assert_eq!(cfg.widget_borders, [1; N]);
        assert_eq!(cfg.widget_opacities, [2; N]);
        assert_eq!(cfg.widget_bars, [2; N]);
        assert_eq!(cfg.widget_subtitles, [1; N]);
        assert_eq!(cfg.positions, positions);
        assert_eq!(cfg.card_sizes, sizes);
        assert_eq!(cfg.enabled, enabled);
    }

    #[test]
    fn invalid_copy_source_does_not_change_visuals() {
        let mut cfg = Config::default();
        let previous = cfg.widget_styles;
        cfg.copy_widget_visual_to_all(N);
        assert_eq!(cfg.widget_styles, previous);
    }
}

#[cfg(test)]
mod rtime_migration_tests {
    use super::*;

    #[test]
    fn new_registry_key_has_priority_over_legacy() {
        assert_eq!(
            registry_read_keys(),
            ["Software\\Rtime", "Software\\RustFetchNative"]
        );
        assert_ne!(KEY, LEGACY_KEY);
    }
}

#[cfg(test)]
mod interface_theme_tests {
    use super::*;
    #[test]
    fn ui_theme_is_independent_from_widget_theme() {
        let mut cfg = Config::default();
        assert_eq!(cfg.interface_theme, 0);
        cfg.interface_theme = 1;
        assert_eq!(cfg.theme, 0);
        assert_eq!(cfg.style, 1);
        assert_eq!(INTERFACE_THEMES, ["Claro", "Escuro"]);
    }
}
