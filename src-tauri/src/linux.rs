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

/// Log de diagnóstico a stderr, activado con la variable APP_MONITOR_DEBUG=1.
/// Útil para diagnosticar por qué un backend no reporta ventanas (p. ej. KDE
/// en Wayland): cada decisión y cada punto de fallo se imprimen en detalle.
fn dbg(msg: impl AsRef<str>) {
    if std::env::var_os("APP_MONITOR_DEBUG").is_some() {
        eprintln!("[app-monitor/linux] {}", msg.as_ref());
    }
}

/// Obtiene la ventana en primer plano (título y proceso) según el entorno.
pub fn get_active_window() -> Option<ActiveWindow> {
    dbg(format!(
        "get_active_window: WAYLAND_DISPLAY={:?} XDG_CURRENT_DESKTOP={:?} DISPLAY={:?} SWAYSOCK={:?}",
        std::env::var_os("WAYLAND_DISPLAY"),
        std::env::var_os("XDG_CURRENT_DESKTOP"),
        std::env::var_os("DISPLAY"),
        std::env::var_os("SWAYSOCK"),
    ));
    let mut result = if is_wayland_session() {
        dbg("entorno: sesion Wayland -> backend wayland");
        wayland::active_window()
    } else {
        dbg("entorno: sesion X11 -> backend x11");
        x11::active_window()
    };

    // Fallback X11/XWayland: en GNOME 46+ (Ubuntu 24.04) tanto
    // Introspect.GetWindows como Shell.Eval están bloqueados por política
    // D-Bus (AccessDenied) para apps de terceros. Si la sesión es Wayland
    // pero el backend nativo no dio ventana y hay un DISPLAY (XWayland),
    // probamos la vía EWMH: al menos las aplicaciones X11/XWayland quedan
    // monitorizadas (las nativas de Wayland no son visibles por X11).
    if result.is_none() && is_wayland_session() && std::env::var_os("DISPLAY").is_some() {
        dbg("wayland sin datos -> probando fallback X11/XWayland (EWMH)");
        result = x11::active_window();
    }

    dbg(format!(
        "get_active_window -> {:?}",
        result.as_ref().map(|w| (&w.title, &w.process_name))
    ));
    result
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
    use super::{dbg, process_name_for_pid, ActiveWindow};
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
                dbg("x11: no se pudo conectar con el servidor X (DISPLAY no disponible?)");
                return None;
            }
        }
        let (conn, root) = guard.as_ref()?;
        match f(conn, *root) {
            Ok(v) => Some(v),
            Err(()) => {
                dbg("x11: error en la consulta de ventana activa");
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
    use super::{dbg, process_name_for_pid, ActiveWindow};

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
        dbg(format!("wayland: XDG_CURRENT_DESKTOP = {desktop:?}"));

        if desktop.contains("gnome") || desktop.contains("unity") {
            dbg("wayland: compositor detectado = GNOME/Unity");
            return gnome::active_window();
        }
        if desktop.contains("sway") || std::env::var_os("SWAYSOCK").is_some() {
            dbg("wayland: compositor detectado = Sway");
            return sway::active_window();
        }
        if desktop.contains("kde") || desktop.contains("plasma") {
            dbg("wayland: compositor detectado = KDE Plasma -> KWin Scripting");
            return kde::active_window();
        }
        dbg("wayland: ningun compositor conocido detectado -> None");
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
        use super::{dbg, process_name_for_pid, ActiveWindow};
        use dbus::blocking::SyncConnection;
        use dbus::channel::MatchingReceiver;
        use dbus::message::MatchRule;
        use serde::Deserialize;
        use std::io::Write;
        use std::sync::atomic::{AtomicU32, Ordering};
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

        /// Backend: conexión de escucha (destino del `callDBus` del script),
        /// su nombre único, una conexión dedicada a KWin para el muestreo y la
        /// caché de la última ventana focada.
        struct Backend {
            conn: SyncConnection,
            unique: String,
            kwin: SyncConnection,
            cache: Arc<Mutex<Option<ActiveWindow>>>,
        }

        static BACKEND: Mutex<Option<Backend>> = Mutex::new(None);
        /// Secuencia para nombres de script únicos (evita "nombre duplicado").
        static SAMPLE_SEQ: AtomicU32 = AtomicU32::new(0);

        pub fn active_window() -> Option<ActiveWindow> {
            // Inicializa el backend KDE la primera vez que se consulta.
            {
                let mut guard = BACKEND.lock().ok()?;
                if guard.is_none() {
                    dbg("kde: backend no inicializado -> intentando spawn()");
                    *guard = spawn();
                    if guard.is_some() {
                        dbg("kde: spawn() OK, backend listo");
                    } else {
                        dbg("kde: spawn() fallo -> backend no disponible (degradando a None)");
                    }
                }
            }

            let backend_guard = BACKEND.lock().ok()?;
            let backend = backend_guard.as_ref();

            // Muestreo activo: en cada consulta cargamos un script de un solo
            // uso que reporta la ventana activa actual (patrón probado por
            // kdotool). No dependemos de señales ni timers del scripting de
            // KWin (varían entre versions de Plasma 6): solo de
            // loadScript/start/unloadScript, que funcionan en todas.
            if let Some(b) = backend {
                sample_once(b);
                // Drena los mensajes "result" pendientes y actualiza la caché.
                let _ = b.conn.process(Duration::from_millis(30));
            }

            let cached = backend?.cache.lock().ok()?;
            if cached.is_none() {
                dbg("kde: cache vacia (KWin no ha reportado ninguna ventana todavia)");
            } else {
                let w = cached.as_ref().unwrap();
                dbg(format!("kde: ventana en cache: title={:?} process={:?}", w.title, w.process_name));
            }
            cached.clone()
        }

        /// Escribe, carga, ejecuta y descarga un script KWin de un solo uso que
        /// reporta la ventana activa en ese momento (título/clase/pid).
        fn sample_once(backend: &Backend) {
            let seq = SAMPLE_SEQ.fetch_add(1, Ordering::Relaxed);
            let script_name = format!("app-monitor-{}-{seq}", std::process::id());
            let script = format!(
                r#"function output_result(message) {{
    callDBus("{}", "/", "", "result", message.toString());
}}
try {{
    var w = workspace.activeWindow;
    if (w) {{
        output_result(JSON.stringify({{
            title: w.caption || "",
            class: w.resourceClass || "",
            pid: (typeof w.pid === "number") ? w.pid : 0
        }}));
    }} else {{
        output_result("null");
    }}
}} catch (e) {{
    output_result(JSON.stringify({{ error: String(e) }}));
}}
"#,
                backend.unique
            );
            let path = std::env::temp_dir().join(format!("{script_name}.js"));
            {
                let mut file = match std::fs::File::create(&path) {
                    Ok(f) => f,
                    Err(e) => {
                        dbg(format!("kde: ERROR escribiendo el script en {path:?}: {e}"));
                        return;
                    }
                };
                if let Err(e) = file.write_all(script.as_bytes()) {
                    dbg(format!("kde: ERROR escribiendo el contenido del script: {e}"));
                    return;
                }
            }

            let proxy = backend
                .kwin
                .with_proxy(KWIN_SERVICE, "/Scripting", Duration::from_millis(5000));

            // 1) Carga.
            let script_result: Result<(i32,), _> = proxy.method_call(
                "org.kde.kwin.Scripting",
                "loadScript",
                (
                    path.to_str().unwrap_or_default(),
                    script_name.as_str(),
                ),
            );
            let script_id = match script_result {
                Ok((id,)) => id,
                Err(e) => {
                    dbg(format!("kde: ERROR loadScript (muestreo): {e}"));
                    let _ = std::fs::remove_file(&path);
                    return;
                }
            };
            if script_id < 0 {
                dbg(format!(
                    "kde: loadScript devolvio script_id={script_id} < 0 -> scripting no disponible o nombre duplicado"
                ));
                let _ = std::fs::remove_file(&path);
                return;
            }

            // 2) Ejecución (Plasma 6 exige start(); en Plasma 5 puede no existir).
            let start_result: Result<(), _> = proxy.method_call("org.kde.kwin.Scripting", "start", ());
            match start_result {
                Ok(()) => {}
                Err(e) => dbg(format!("kde: start() no disponible (normal en Plasma 5): {e}")),
            }

            // 3) Da tiempo a que el script se ejecute y llegue el "result".
            let _ = backend.conn.process(Duration::from_millis(30));

            // 4) Descarga el script para no dejar acumulados.
            let unload_result: Result<(bool,), _> = proxy.method_call(
                "org.kde.kwin.Scripting",
                "unloadScript",
                (script_name.as_str(),),
            );
            match unload_result {
                Ok((removed,)) => {
                    if removed {
                        dbg(format!("kde: unloadScript({script_name}) OK"));
                    } else {
                        dbg(format!(
                            "kde: unloadScript({script_name}) -> false (no estaba cargado)"
                        ));
                    }
                }
                Err(e) => dbg(format!("kde: unloadScript fallo (normal en Plasma 5): {e}")),
            }

            let _ = std::fs::remove_file(&path);
        }

        fn spawn() -> Option<Backend> {
            // 1) Conexión de escucha; su nombre único será el destino del script.
            let conn = match SyncConnection::new_session() {
                Ok(c) => c,
                Err(e) => {
                    dbg(format!("kde: ERROR conectando al bus de sesion D-Bus: {e}"));
                    return None;
                }
            };
            let unique = conn.unique_name().to_string();
            dbg(format!("kde: conexion de escucha OK, unique name = {unique}"));
            let cache = Arc::new(Mutex::new(None::<ActiveWindow>));

            let cache_rx = cache.clone();
            let receiver = conn.start_receive(
                MatchRule::new_method_call(),
                Box::new(move |message, _connection| {
                    if let Some(member) = message.member() {
                        if let Some(arg) = message.get1::<String>() {
                            if &*member == "result" {
                                if let Some(win) = parse_payload(&arg) {
                                    dbg(format!(
                                        "kde: KWin reporto ventana via callDBus -> title={:?} process={:?}",
                                        win.title, win.process_name
                                    ));
                                    *cache_rx.lock().unwrap() = Some(win);
                                } else {
                                    dbg(format!("kde: KWin envio payload no parseable: {arg:?}"));
                                }
                            } else {
                                dbg(format!("kde: metodo D-Bus inesperado: {member:?}"));
                            }
                        }
                    }
                    true // mantener el listener
                }),
            );
            // El receptor vive mientras el proceso (backend de app-lifetime).
            Box::leak(Box::new(receiver));

            // 2) Conexión D-Bus dedicada a KWin (para loadScript/start/unload).
            let kwin = match SyncConnection::new_session() {
                Ok(c) => c,
                Err(e) => {
                    dbg(format!("kde: ERROR segunda conexion al bus de sesion: {e}"));
                    return None;
                }
            };

            dbg("kde: backend KWin inicializado correctamente");
            Some(Backend { conn, unique, kwin, cache })
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
        use super::{dbg, resolve_process, ActiveWindow};
        use std::collections::HashMap;
        use zbus::blocking::{Connection, Proxy};
        use zbus::zvariant::{OwnedValue, Value};

        fn session() -> Option<Connection> {
            Connection::session().ok()
        }

        pub fn active_window() -> Option<ActiveWindow> {
            let conn = session()?;

            // 1) org.gnome.Shell.Introspect.GetWindows.
            if let Some(win) = introspect_window(&conn) {
                return Some(win);
            }

            // 2) Fallback: org.gnome.Shell.Eval (disponible en algunas distros;
            //    en Ubuntu 24.04 está restringido por política D-Bus).
            if let Some(win) = eval_window(&conn) {
                return Some(win);
            }

            dbg("gnome: sin ventana activa (Introspect y Eval agotados)");
            None
        }

        /// GNOME <= 45 y 46+: org.gnome.Shell.Introspect.GetWindows.
        /// OJO (GNOME 46): el método ya NO acepta la opción a{sv} (firma "()");
        /// devuelve todas las ventanas y el foco se filtra por el campo "focus".
        fn introspect_window(conn: &Connection) -> Option<ActiveWindow> {
            let proxy = match Proxy::new(
                conn,
                "org.gnome.Shell",
                "/org/gnome/Shell/Introspect",
                "org.gnome.Shell.Introspect",
            ) {
                Ok(p) => p,
                Err(e) => {
                    dbg(format!("gnome: Introspect no disponible: {e}"));
                    return None;
                }
            };

            // Sin opciones (GNOME 46+); devuelve todas las ventanas.
            let windows: Vec<HashMap<String, OwnedValue>> = match proxy.call("GetWindows", &()) {
                Ok(w) => w,
                Err(e) => {
                    dbg(format!("gnome: Introspect.GetWindows fallo: {e}"));
                    return None;
                }
            };
            dbg(format!(
                "gnome: Introspect devolvio {} ventanas",
                windows.len()
            ));
            if windows.is_empty() {
                dbg("gnome: Introspect sin ventanas");
                return None;
            }

            // Busca la ventana marcada con foco (campo "focus": true).
            let focused = windows
                .iter()
                .find(|w| match w.get("focus").map(|v| &**v) {
                    Some(Value::Bool(b)) => *b,
                    _ => false,
                });
            // Si ninguna lo está, loguea las claves de la primera (útil para
            // saber el esquema real sin recompilar) y usa la primera.
            let win = match focused.or_else(|| windows.first()) {
                Some(w) => w,
                None => return None,
            };
            if focused.is_none() {
                let keys: Vec<&String> = win.keys().collect();
                dbg(format!("gnome: sin ventana con focus; claves: {keys:?}"));
            }
            let title = str_field(win, "title").unwrap_or_default();
            let pid = int_field(win, "pid");
            let class = str_field(win, "class").or_else(|| str_field(win, "app-id"));
            let process_name = resolve_process(pid, class).unwrap_or_default();
            dbg(format!(
                "gnome: Introspect OK -> title={title:?} process={process_name:?}"
            ));
            Some(ActiveWindow { title, process_name })
        }

        /// Fallback alternativo: org.gnome.Shell.Eval — evalúa JS en el shell y
        /// devuelve el valor de la última expresión. Disponible en algunas
        /// distros configuraciones; en Ubuntu 24.04 la política D-Bus de
        /// gnome-shell lo restringe (responde (false, "")), por lo que este
        /// fallback suele quedar sin efecto ahí.
        fn eval_window(conn: &Connection) -> Option<ActiveWindow> {
            let proxy = match Proxy::new(
                conn,
                "org.gnome.Shell",
                "/org/gnome/Shell",
                "org.gnome.Shell",
            ) {
                Ok(p) => p,
                Err(e) => {
                    dbg(format!("gnome: proxy Shell no disponible: {e}"));
                    return None;
                }
            };

            let script = r#"(() => {
  const w = global.display.focus_window;
  if (!w) return "null";
  return JSON.stringify({
    title: String(w.title || ""),
    class: String((w.get_wm_class ? w.get_wm_class() : "") || ""),
    pid: (w.get_pid ? w.get_pid() : 0)
  });
})()"#;

            let result: Result<(bool, String), _> = proxy.call("Eval", &(script,));
            let (ok, out) = match result {
                Ok(v) => v,
                Err(e) => {
                    dbg(format!("gnome: Eval fallo (puede requerir permisos): {e}"));
                    return None;
                }
            };
            if !ok {
                dbg(format!("gnome: Eval devolvio error: {out:?}"));
                return None;
            }
            let payload = out.trim();
            if payload == "null" || payload.is_empty() {
                dbg("gnome: Eval -> sin ventana focada");
                return None;
            }

            // Eval devuelve el JSON entre comillas; lo extraemos con un parse
            // tolerante buscando el primer '{' y el último '}'.
            let start = payload.find('{')?;
            let end = payload.rfind('}')?;
            let json: serde_json::Value = serde_json::from_str(&payload[start..=end]).ok()?;

            let title = json
                .get("title")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let class = json
                .get("class")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let pid = json.get("pid").and_then(serde_json::Value::as_u64).map(|p| p as u32);
            let process_name = resolve_process(pid, Some(class)).unwrap_or_default();
            dbg(format!(
                "gnome: Eval OK -> title={title:?} process={process_name:?}"
            ));
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

        /// Árbol de ejemplo para el test: pid 0 nunca resuelve un proceso real,
        /// así que la resolución cae al app_id ("foot") de forma determinista.
        #[cfg(test)]
        pub(crate) fn sample_tree() -> Json {
            serde_json::json!({
                "name": "root",
                "focused": false,
                "nodes": [
                    {
                        "name": "Terminal",
                        "app_id": "foot",
                        "pid": 0,
                        "focused": true,
                        "nodes": []
                    },
                    { "name": "Browser", "app_id": "firefox", "pid": 0, "focused": false, "nodes": [] }
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
        let win = wayland::kde::parse_payload(r#"{"title":"Konsole","class":"konsole","pid":0}"#);
        assert!(win.is_some());
        let win = win.unwrap();
        assert_eq!(win.title, "Konsole");
        // pid 0 nunca resuelve un proceso real -> cae a la clase de la ventana.
        assert_eq!(win.process_name, "konsole");
    }

    #[test]
    fn kde_parses_null_payload_as_none() {
        assert!(wayland::kde::parse_payload("null").is_none());
    }
}