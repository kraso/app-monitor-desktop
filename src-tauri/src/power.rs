//! Detección de bloqueo y suspensión del sistema (solo Windows).
//!
//! Fase 3: el plan exige detectar bloqueo y suspensión para cerrar la
//! sesión activa. Usamos una ventana de mensajes oculta (clase "STATIC",
//! ya registrada por el sistema) que recibe:
//! - `WM_WTSSESSION_CHANGE` (WTS_LOCK/UNLOCK) vía `WTSRegisterSessionNotification`.
//! - `WM_POWERBROADCAST` (suspend/resume).

#![cfg(windows)]

use std::sync::mpsc::Sender;
use std::thread;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::RemoteDesktop::{
    WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
    NOTIFY_FOR_THIS_SESSION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, SetWindowLongPtrW, TranslateMessage, CW_USEDEFAULT, GWLP_USERDATA,
    GWLP_WNDPROC, WM_DESTROY, WM_POWERBROADCAST, WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK,
    WTS_SESSION_UNLOCK,
};

/// Eventos de energía/bloqueo que interesan al monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerEvent {
    Lock,
    Unlock,
    Suspend,
    Resume,
}

const STATIC_CLASS: [u16; 7] = [b'S' as u16, b'T' as u16, b'A' as u16, b'T' as u16, b'I' as u16, b'C' as u16, 0];

/// Arranca un hilo que escucha eventos de bloqueo/suspensión y los envía
/// por el canal `tx`.
pub fn spawn_power_listener(tx: Sender<PowerEvent>) {
    thread::spawn(move || {
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                STATIC_CLASS.as_ptr(),
                std::ptr::null(),
                0,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                0,
                0,
                0,
                std::ptr::null(),
            );
            if hwnd == 0 {
                return;
            }

            WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION as u32);

            // Sustituimos el proc de la ventana por el nuestro para capturar
            // los mensajes de sesión/energía.
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);

            // Guardamos el sender en el user-data de la ventana.
            let boxed = Box::into_raw(Box::new(tx.clone()));
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, boxed as isize);

            let mut msg = std::mem::zeroed();
            while GetMessageW(&mut msg, 0, 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            WTSUnRegisterSessionNotification(hwnd);
            let _ = DestroyWindow(hwnd);
        }
    });
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_WTSSESSION_CHANGE => match wparam as u32 {
            WTS_SESSION_LOCK => forward(hwnd, PowerEvent::Lock),
            WTS_SESSION_UNLOCK => forward(hwnd, PowerEvent::Unlock),
            _ => {}
        },
        WM_POWERBROADCAST => {
            const PBT_APMSUSPEND: WPARAM = 4;
            const PBT_APMRESUMESUSPEND: WPARAM = 7;
            match wparam {
                PBT_APMSUSPEND => forward(hwnd, PowerEvent::Suspend),
                PBT_APMRESUMESUSPEND => forward(hwnd, PowerEvent::Resume),
                _ => {}
            }
        }
        WM_DESTROY => {}
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe fn forward(hwnd: HWND, ev: PowerEvent) {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if ptr != 0 {
        let tx = &*(ptr as *const Sender<PowerEvent>);
        let _ = tx.send(ev);
    }
}
