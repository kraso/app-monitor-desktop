//! Acceso a la ventana activa y a la inactividad del sistema (solo Windows).
//!
//! Fase 3: captura de la ventana en primer plano, su proceso asociado y el
//! tiempo de inactividad del usuario. Usa `windows-sys` 0.52. `LASTINPUTINFO`
//! y `GetLastInputInfo` se declaran manualmente porque esta versión no los
//! expone bajo `SystemInformation`.

#![cfg(windows)]

use std::mem::MaybeUninit;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::ProcessStatus::{
    GetModuleBaseNameW, K32GetModuleFileNameExW,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
};

/// Estructura de la Win32 API para el tiempo de inactividad.
#[repr(C)]
struct LastInputInfo {
    cb_size: u32,
    dw_time: u32,
}

extern "system" {
    fn GetLastInputInfo(plii: *mut LastInputInfo) -> i32;
    fn GetTickCount() -> u32;
}

/// Información de la ventana actualmente en primer plano.
#[derive(Debug, Clone)]
pub struct ActiveWindow {
    pub title: String,
    pub process_name: String,
}

/// Obtiene la ventana en primer plano: título, PID y nombre del proceso.
pub fn get_active_window() -> Option<ActiveWindow> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 {
            return None;
        }

        let len = GetWindowTextLengthW(hwnd) as usize;
        let title = if len > 0 {
            let mut buf = vec![0u16; len + 1];
            let read = GetWindowTextW(hwnd, buf.as_mut_ptr(), (len + 1) as i32);
            if read > 0 {
                String::from_utf16_lossy(&buf[..read as usize])
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid as *mut u32);
        if pid == 0 {
            return None;
        }

        let process_name = process_name_for_pid(pid).unwrap_or_default();
        Some(ActiveWindow {
            title,
            process_name,
        })
    }
}

/// Devuelve el nombre del ejecutable (p. ej. "chrome.exe") para un PID.
fn process_name_for_pid(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid);
        if handle == 0 {
            return None;
        }
        let _guard = ProcessGuard(handle);

        let mut buf = [0u16; 1024];
        let len = GetModuleBaseNameW(handle, 0, buf.as_mut_ptr(), buf.len() as u32);
        if len > 0 {
            return Some(String::from_utf16_lossy(&buf[..len as usize]));
        }
        let len = K32GetModuleFileNameExW(handle, 0, buf.as_mut_ptr(), buf.len() as u32);
        if len > 0 {
            let full = String::from_utf16_lossy(&buf[..len as usize]);
            return Some(full.rsplit('\\').next().unwrap_or(&full).to_string());
        }
        None
    }
}

/// Tiempo (ms) desde la última entrada del usuario (teclado/ratón).
pub fn idle_time_ms() -> u64 {
    unsafe {
        let mut li = LastInputInfo {
            cb_size: std::mem::size_of::<LastInputInfo>() as u32,
            dw_time: 0,
        };
        if GetLastInputInfo(&mut li as *mut LastInputInfo) == 0 {
            return 0;
        }
        let now = GetTickCount();
        now.saturating_sub(li.dw_time) as u64
    }
}

struct ProcessGuard(HANDLE);

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[allow(dead_code)]
fn _unused(_: MaybeUninit<u8>) {}
