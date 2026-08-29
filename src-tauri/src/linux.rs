//! Acceso a la ventana activa y a la inactividad del sistema (solo Linux).
//!
//! Fase 5: backend nativo de monitorización para Linux.
//!
//! Estrategia: se detecta el entorno en tiempo de ejecución y se usa el
//! backend correcto; si no hay ninguno disponible, las funciones devuelven
//! `None`/`0` y la app sigue funcionando (sin datos de monitorización).
//!
//! - **X11** (cualquier escritorio con servidor X): ventana activa vía
//!   `_NET_ACTIVE_WINDOW` (EWMH) e inactividad vía la extensión XScreenSaver.
//!   Implementado con `x11rb` (protocolo X puro en Rust, sin libs de C).
//! - **Wayland / GNOME Shell**: ventana focada vía `org.gnome.Shell.Introspect`
//!   e inactividad vía `org.gnome.Mutter.IdleMonitor` (D-Bus, `zbus`).
//! - **Wayland / Sway (wlroots)**: ventana focada vía el IPC de sway
//!   (`$SWAYSOCK`, petición `get_tree`).
//! - **Wayland / KDE Plasma**: ventana focada vía KWin Scripting — el script
//!   reporta `workspace.activeWindow` (título/clase/pid) por `callDBus` en
//!   cada cambio de foco (protocolo probado por kdotool).

#![cfg(target_os = "linux")]

/// Información de la ventana actualmente en primer plano.
#[derive(Debug, Clone)]
pub struct ActiveWindow {
    pub title: String,
    pub process_name: String,
}

/// Obtiene la ventana en primer plano (título y proceso) según el entorno.
pub fn get_active_window() -> Option<ActiveWindow> {
    if is_wayland_session() {
        wayland::active_window()
    } else {
        x11::active_window()
    }
}

/// Milisegundos desde la última interacción del usuario (teclado/ratón).
pub fn idle_time_ms() -> u64 {
    if is_wayland_session() {
        wayland::idle_time_ms()
    } else {
        x11::idle_time_ms()
    }
}

/// `true` cuando la sesión es Wayland (los clientes X11 de XWayland siguen
/// existiendo, pero la ventana focada nativa solo la conoce el compositor).
fn is_wayland_session() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// Nombre del proceso (sin ruta ni extensión) a partir del PID, leyendo /proc.
fn process_name_for_pid(pid: u32) -> Option<String> {
    if pid == 0 {
        return None;
    }
    if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
        let name = comm.trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    std::fs::read_link(format!("/proc/{pid}/exe"))
        .ok()
        .and_then(|p| p.file_name().map(|f| f.to_string_lossy().into_owned()))
}

/// ===========================================================================
/// Backend X11 (EWMH + XScreenSaver) — cualquier escritorio con servidor X.
/// ===========================================================================
mod x11 {
    use super::{process_name_for_pid, ActiveWindow};
    use std::sync::Mutex;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};
    use x11rb::rust_connection::RustConnection;

    /// Conexión X11 cacheada; si el servidor X se reinicia, la siguiente
    /// llamada reconecta automáticamente.
    static CONN: Mutex<Option<(RustConnection, u32)>> = Mutex::new(None);

    fn with_x11<T>(f: impl FnOnce(&RustConnection, u32) -> Result<T, ()>) -> Option<T> {
        let mut guard = CONN.lock().ok()?;
        if guard.is_none() {
            if let Ok((conn, screen)) = RustConnection::connect(None) {
                let root = conn.setup().roots[screen].root;
                *guard = Some((conn, root));
            } else {
                return None;
            }
        }
        let (conn, root) = guard.as_ref()?;
        match f(conn, *root) {
            Ok(v) => Some(v),
            Err(()) => {
                *guard = None; // el servidor X cayó: reconectar la próxima vez
                None
            }
        }
    }

    fn intern(conn: &RustConnection, name: &[u8]) -> Result<u32, ()> {
        let reply = conn
            .intern_atom(false, name)
            .map_err(|_| ())?
            .reply()
            .map_err(|_| ())?;
        Ok(reply.atom.into())
    }

    pub fn active_window() -> Option<ActiveWindow> {
        with_x11(|conn, root| {
            let a_active = intern(conn, b"_NET_ACTIVE_WINDOW")?;
            let a_pid = intern(conn, b"_NET_WM_PID")?;
            let a_title = intern(conn, b"_NET_WM_NAME")?;
            let a_title_old = intern(conn, b"WM_NAME")?;

            // 1) Ventana activa (EWMH) desde la raíz.
            let prop = conn
                .get_property(false, root, a_active, AtomEnum::WINDOW, 0, 1)
                .map_err(|_| ())?
                .reply()
                .map_err(|_| ())?;
            if prop.value.len() < 4 {
                return Err(());
            }
            let window = u32::from_le_bytes([prop.value[0], prop.value[1], prop.value[2], prop.value[3]]);
            if window == 0 {
                return Err(());
            }

            // 2) Título (_NET_WM_NAME UTF-8, con fallback a WM_NAME).
            let title = window_text(conn, window, a_title, a_title_old).unwrap_or_default();

            // 3) Proceso: PID (_NET_WM_PID) -> /proc, con fallbacks a "unknown".
            let pid = get_u32(conn, window, a_pid, AtomEnum::CARDINAL);
            let process_name = pid
                .and_then(process_name_for_pid)
                .unwrap_or_else(|| String::from("unknown"));

            Ok(ActiveWindow { title, process_name })
        })
    }

    pub fn idle_time_ms() -> u64 {
        with_x11(|conn, root| {
            // Extensión XScreenSaver -> ms desde la última entrada del usuario.
            let reply = x11rb::protocol::screensaver::query_info(conn, root)
                .map_err(|_| ())?
                .reply()
                .map_err(|_| ())?;
            Ok(u64::from(reply.ms_since_user_input))
        })
        .unwrap_or(0)
    }

    fn window_text(
        conn: &RustConnection,
        window: u32,
        a_utf8: u32,
        a_fallback: u32,
    ) -> Option<String> {
        let p = conn
            .get_property(false, window, a_utf8, AtomEnum::ANY, 0, u32::MAX)
            .ok()?
            .reply()
            .ok()?;
        if !p.value.is_empty() {
            return Some(String::from_utf8_lossy(&p.value).trim_end_matches('\0').to_string());
        }
        let p = conn
            .get_property(false, window, a_fallback, AtomEnum::ANY, 0, u32::MAX)
            .ok()?
            .reply()
            .ok()?;
        if !p.value.is_empty() {
            Some(String::from_utf8_lossy(&p.value).trim_end_matches('\0').to_string())
        } else {
            None
        }
    }

    fn get_u32(
        conn: &RustConnection,
        window: u32,
        atom: u32,
        ty: AtomEnum,
    ) -> Option<u32> {
        let p = conn
            .get_property(false, window, atom, ty, 0, 1)
            .ok()?
            .reply()
            .ok()?;
        if p.value.len() >= 4 {
            Some(u32::from_le_bytes([p.value[0], p.value[1], p.value[2], p.value[3]]))
        } else {
            None
        }
    }
}

/// ===========================================================================
/// Backend Wayland — por compositor (GNOME, Sway, KDE).
/// ===========================================================================
mod wayland {
    use super::{process_name_for_pid, ActiveWindow};

    /// El nombre del PID siempre se prefiere a la clase de la ventana.
    fn resolve_process(pid: Option<u32>, class: Option<String>) -> Option<String> {
        pid.and_then(process_name_for_pid)
            .or(class)
            .or_else(|| Some(String::from("unknown")))
    }

    pub fn active_window() -> Option<ActiveWindow> {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_lowercase();

        if desktop.contains("gnome") || desktop.contains("unity") {
            return gnome::active_window();
        }
        if desktop.contains("sway") || std::env::var_os("SWAYSOCK").is_some() {
            return sway::active_window();
        }
        if desktop.contains("kde") || desktop.contains("plasma") {
            return kde::active_window();
        }
        None
    }

    pub fn idle_time_ms() -> u64 {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_lowercase();
        if desktop.contains("gnome") || desktop.contains("unity") {
            return gnome::idle_time_ms();
        }
        0 // GNOME no presente: sin API de idle por ahora -> nunca "idle"
    }

    /// -----------------------------------------------------------------------
    /// KDE Plasma (Wayland): KWin Scripting con reporte por evento.
    /// -----------------------------------------------------------------------
    /// Protocolo probado en producción por kdotool: abrimos una conexión D-Bus
    /// de sesión y usamos su *nombre único* como destino del `callDBus` del
    /// script de KWin. El script lee `workspace.activeWindow` y envía
    /// título/clase/pid en cada `activeWindowChanged` (y al cargar). Se carga
    /// una sola vez y nuestra conexión recibe los mensajes "result".
    ///
    /// Requisito de validación en vivo: KDE Plasma 6 en sesión Wayland con
    /// scripting de KWin activo. Si KWin no permite cargar el script (script_id
    /// < 0), degradamos a `None` y la app sigue funcionando sin datos.
    /// -----------------------------------------------------------------------
    pub(crate) mod kde {
        use super::{process_name_for_pid, ActiveWindow};
        use dbus::blocking::SyncConnection;
        use dbus::channel::{Channel, MatchRule};
        use serde::Deserialize;
        use std::io::Write;
        use std::sync::{Arc, Mutex};
        use std::time::Duration;

        const KWIN_SERVICE: &str = "org.kde.KWin";

        /// Payload JSON que envía el script de KWin.
        #[derive(Deserialize)]
        struct WindowInfo {
            title: Option<String>,
            #[serde(rename = "class")]
            class: Option<String>,
            pid: Option<u32>,
        }

        /// Conexión receptora + caché de la última ventana focada notificada.
        struct Backend {
            conn: SyncConnection,
            cache: Arc<Mutex<Option<ActiveWindow>>>,
        }

        static BACKEND: Mutex<Option<Backend>> = Mutex::new(None);

        pub fn active_window() -> Option<ActiveWindow> {
            // Inicializa el backend KDE la primera vez que se consulta.
            {
                let mut guard = BACKEND.lock().ok()?;
                if guard.is_none() {
                    *guard = spawn();
                }
            }
            let backend_guard = BACKEND.lock().ok()?;
            let backend = backend_guard.as_ref()?;
            // Drena los mensajes "result" pendientes y actualiza la caché.
            let _ = backend.conn.process(Duration::from_millis(10));
            let cached = backend.cache.lock().ok()?;
            cached.clone()
        }

        fn spawn() -> Option<Backend> {
            // 1) Conexión de escucha; su nombre único será el destino del script.
            let conn = SyncConnection::new_session().ok()?;
            let unique = conn.unique_name().to_string();
            let cache = Arc::new(Mutex::new(None::<ActiveWindow>));

            let cache_rx = cache.clone();
            let receiver = conn.start_receive(
                MatchRule::new_method_call(),
                Box::new(move |message, _connection| {
                    if let Some(member) = message.member() {
                        if let Some(arg) = message.get1::<String>() {
                            if member.as_ref() == "result" {
                                if let Some(win) = parse_payload(&arg) {
                                    *cache_rx.lock().unwrap() = Some(win);
                                }
                            }
                        }
                    }
                    true // mantener el listener
                }),
            );
            // El receptor vive mientras el proceso (backend de app-lifetime).
            Box::leak(Box::new(receiver));

            // 2) Script de KWin: reporta al cargar y en cada cambio de foco.
            let script = format!(
                r#"function output_result(message) {{
    callDBus("{}", "/", "", "result", message.toString());
}}
function report() {{
    const w = workspace.activeWindow;
    if (w) {{
        output_result(JSON.stringify({{ title: w.caption, class: w.resourceClass, pid: w.pid }}));
    }} else {{
        output_result("null");
    }}
}}
workspace.activeWindowChanged.connect(report);
report();
"#,
                unique
            );

            let script_name = format!("app-monitor-{}", std::process::id());
            let path = std::env::temp_dir().join(format!("{script_name}.js"));
            {
                let mut file = std::fs::File::create(&path).ok()?;
                file.write_all(script.as_bytes()).ok()?;
            }

            // 3) Carga del script en KWin (org.kde.KWin /Scripting).
            let kwin = SyncConnection::new_session().ok()?;
            let proxy = kwin.with_proxy(KWIN_SERVICE, "/Scripting", Duration::from_millis(5000));
            let (script_id,): (i32,) = proxy
                .method_call(
                    "org.kde.kwin.Scripting",
                    "loadScript",
                    (
                        path.to_str().unwrap_or_default(),
                        script_name.as_str(),
                    ),
                )
                .ok()?;
            if script_id < 0 {
                return None; // scripting de KWin no disponible / nombre duplicado
            }
            // KWin ya leyó el fichero en loadScript (como hace kdotool).
            let _ = std::fs::remove_file(&path);

            Some(Backend { conn, cache })
        }

        /// Parsea el payload JSON enviado por el script de KWin.
        pub(crate) fn parse_payload(payload: &str) -> Option<ActiveWindow> {
            if payload == "null" {
                return None;
            }
            let info: WindowInfo = serde_json::from_str(payload).ok()?;
            let title = info.title.unwrap_or_default();
            let process_name = info
                .pid
                .and_then(process_name_for_pid)
                .or(info.class)
                .unwrap_or_else(|| String::from("unknown"));
            Some(ActiveWindow { title, process_name })
        }
    }

    /// -----------------------------------------------------------------------
    /// GNOME Shell (X11 o Wayland): Introspect + Mutter IdleMonitor.
    /// -----------------------------------------------------------------------
    mod gnome {
        use super::{resolve_process, ActiveWindow};
        use std::collections::HashMap;
        use zbus::blocking::{Connection, Proxy};
        use zbus::zvariant::{OwnedValue, Value};

        fn session() -> Option<Connection> {
            Connection::session().ok()
        }

        pub fn active_window() -> Option<ActiveWindow> {
            let conn = session()?;
            let proxy = Proxy::new(
                &conn,
                "org.gnome.Shell",
                "/org/gnome/Shell/Introspect",
                "org.gnome.Shell.Introspect",
            )
            .ok()?;

            // Con la opción "focus", GNOME devuelve solo la ventana con foco.
            let mut options = HashMap::new();
            options.insert("focus".to_string(), Value::Bool(true));
            let windows: Vec<HashMap<String, OwnedValue>> =
                proxy.call("GetWindows", &(&options,)).ok()?;

            let win = windows.into_iter().next()?;
            let title = str_field(&win, "title").unwrap_or_default();
            let pid = int_field(&win, "pid");
            let class = str_field(&win, "class")
                .or_else(|| str_field(&win, "app-id"));
            let process_name = resolve_process(pid, class).unwrap_or_default();

            Some(ActiveWindow { title, process_name })
        }

        pub fn idle_time_ms() -> u64 {
            let conn = match session() {
                Some(c) => c,
                None => return 0,
            };
            let proxy = match Proxy::new(
                &conn,
                "org.gnome.Mutter.IdleMonitor",
                "/org/gnome/Mutter/IdleMonitor/Core",
                "org.gnome.Mutter.IdleMonitor",
            ) {
                Ok(p) => p,
                Err(_) => return 0,
            };
            let ms: u32 = proxy.call("GetIdletime", &(0i32,)).unwrap_or(0);
            u64::from(ms)
        }

        fn str_field(f: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
            // OwnedValue deref a Value<'static>; se accede por deref.
            match f.get(key).map(|v| &**v) {
                Some(Value::Str(s)) => Some(s.to_string()),
                _ => None,
            }
        }

        fn int_field(f: &HashMap<String, OwnedValue>, key: &str) -> Option<u32> {
            match f.get(key).map(|v| &**v) {
                Some(Value::U32(n)) => Some(*n),
                Some(Value::U64(n)) => Some(*n as u32),
                Some(Value::I32(n)) if *n >= 0 => Some(*n as u32),
                _ => None,
            }
        }
    }

    /// -----------------------------------------------------------------------
    /// Sway (wlroots): IPC de sway sobre $SWAYSOCK (petición get_tree).
    /// -----------------------------------------------------------------------
    pub(crate) mod sway {
        use super::{process_name_for_pid, ActiveWindow};
        use serde_json::Value as Json;
        use std::io::{Read, Write};
        use std::os::unix::net::UnixStream;

        // Cabecera del protocolo IPC de i3/sway: magic "i3-ipc" + len (LE) + tipo.
        const TYPE_GET_TREE: u8 = 4;

        pub fn active_window() -> Option<ActiveWindow> {
            let sock = std::env::var("SWAYSOCK").ok()?;
            let mut stream = UnixStream::connect(&sock).ok()?;

            let body: &[u8] = &[];
            let mut header = Vec::with_capacity(14);
            header.extend_from_slice(b"i3-ipc");
            header.extend_from_slice(&(body.len() as u32).to_le_bytes());
            header.push(TYPE_GET_TREE);
            stream.write_all(&header).ok()?;

            let mut meta = [0u8; 14];
            stream.read_exact(&mut meta).ok()?;
            if &meta[..6] != b"i3-ipc" {
                return None;
            }
            let len = u32::from_le_bytes([meta[6], meta[7], meta[8], meta[9]]) as usize;
            let mut payload = vec![0u8; len];
            stream.read_exact(&mut payload).ok()?;

            let tree: Json = serde_json::from_slice(&payload).ok()?;
            parse_focused(&tree)
        }

        pub(crate) fn parse_focused(node: &Json) -> Option<ActiveWindow> {
            if node.get("focused").and_then(Json::as_bool) == Some(true) {
                let pid = node.get("pid").and_then(Json::as_u64).map(|p| p as u32);
                let class = node
                    .get("app_id")
                    .and_then(Json::as_str)
                    .map(String::from)
                    .or_else(|| node.get("name").and_then(Json::as_str).map(String::from));
                let title = node
                    .get("name")
                    .and_then(Json::as_str)
                    .map(String::from)
                    .unwrap_or_default();
                let process_name = pid
                    .and_then(process_name_for_pid)
                    .or(class)
                    .unwrap_or_else(|| String::from("unknown"));
                return Some(ActiveWindow { title, process_name });
            }
            for key in ["nodes", "floating_nodes"] {
                if let Some(list) = node.get(key).and_then(Json::as_array) {
                    for child in list {
                        if let Some(w) = parse_focused(child) {
                            return Some(w);
                        }
                    }
                }
            }
            None
        }

        #[cfg(test)]
        pub(crate) fn sample_tree() -> Json {
            serde_json::json!({
                "name": "root",
                "focused": false,
                "nodes": [
                    {
                        "name": "Terminal",
                        "app_id": "foot",
                        "pid": 4242,
                        "focused": true,
                        "nodes": []
                    },
                    { "name": "Browser", "app_id": "firefox", "pid": 100, "focused": false, "nodes": [] }
                ],
                "floating_nodes": []
            })
        }

        #[cfg(test)]
        pub(crate) fn empty_tree() -> Json {
            serde_json::json!({ "name": "root", "focused": false, "nodes": [], "floating_nodes": [] })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_name_from_pid_of_current_process() {
        // /proc/<pid>/comm existe para nuestro propio proceso en Linux.
        let name = process_name_for_pid(std::process::id());
        assert!(name.is_some() && !name.unwrap().is_empty());
    }

    #[test]
    fn swaps_wayland_ignores_missing_display() {
        // Sin WAYLAND_DISPLAY ni DISPLAY, ambas rutas degradan sin romper.
        std::env::remove_var("WAYLAND_DISPLAY");
        assert!(x11::active_window().is_none() || std::env::var_os("DISPLAY").is_some());
        assert_eq!(x11::idle_time_ms(), 0);
    }

    #[test]
    fn sway_tree_parses_focused_window() {
        let tree = wayland::sway::sample_tree();
        let win = wayland::sway::parse_focused(&tree);
        assert!(win.is_some());
        let win = win.unwrap();
        assert_eq!(win.title, "Terminal");
        assert_eq!(win.process_name, "foot");
    }

    #[test]
    fn sway_tree_empty_returns_none() {
        let tree = wayland::sway::empty_tree();
        assert!(wayland::sway::parse_focused(&tree).is_none());
    }

    #[test]
    fn kde_parses_window_payload() {
        let win = wayland::kde::parse_payload(r#"{"title":"Konsole","class":"konsole","pid":1234}"#);
        assert!(win.is_some());
        let win = win.unwrap();
        assert_eq!(win.title, "Konsole");
        // pid 1234 no existe -> cae a la clase de la ventana.
        assert_eq!(win.process_name, "konsole");
    }

    #[test]
    fn kde_parses_null_payload_as_none() {
        assert!(wayland::kde::parse_payload("null").is_none());
    }
}