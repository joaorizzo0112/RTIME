//! Obtenção dos monitores e sua área útil pelas APIs Win32 oficiais.
use std::ptr::null;
use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{LPARAM, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, MONITORINFOF_PRIMARY, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use super::layout::{Monitor, Rect};

fn rectangle(win: RECT) -> Rect {
    Rect {
        left: win.left,
        top: win.top,
        right: win.right,
        bottom: win.bottom,
    }
}

unsafe extern "system" fn callback(
    handle: HMONITOR,
    _device_context: HDC,
    _area: *mut RECT,
    context: LPARAM,
) -> BOOL {
    let result = &mut *(context as *mut Vec<Monitor>);
    let mut info: MONITORINFO = std::mem::zeroed();
    info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    if GetMonitorInfoW(handle, &mut info) == 0 {
        return 1;
    }
    let mut dpi_x = 96u32;
    let mut dpi_y = 96u32;
    if GetDpiForMonitor(handle, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) != 0 {
        dpi_x = 96;
    }
    result.push(Monitor {
        work: rectangle(info.rcWork),
        screen: rectangle(info.rcMonitor),
        dpi: dpi_x.clamp(72, 384),
        primary: info.dwFlags & MONITORINFOF_PRIMARY != 0,
    });
    1
}

pub fn monitors() -> Vec<Monitor> {
    let mut list = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            null(),
            Some(callback),
            (&mut list as *mut Vec<Monitor>) as isize,
        );
    }

    if list.is_empty() {
        let (x, y, w, h) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1),
                GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1),
            )
        };
        list.push(Monitor {
            work: Rect::new(x, y, w, h),
            screen: Rect::new(x, y, w, h),
            dpi: 96,
            primary: true,
        });
    }
    list.sort_by_key(|monitor| !monitor.primary);
    list
}
