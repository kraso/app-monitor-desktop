# App Monitor

Aplicación de escritorio que registra cuánto tiempo se usa cada aplicación del
ordenador: sesiones de uso (ventana activa + proceso), inactividad, historial
y estadísticas por aplicación — **todo guardado en local, sin servidor ni
internet**.

- **UI / Dashboard:** Next.js 16 (App Router, export estático) + React 19 + TypeScript + Tailwind v4.
- **Agente nativo:** Tauri 2 + Rust (acceso al sistema operativo).
- **Persistencia:** SQLite embebido + Drizzle ORM (los datos sobreviven a reinicios).

## Estado actual

Monitorización de sesiones operativa con persistencia local:

- Cambio de ventana/ventana activa → se cierra una sesión y abre la siguiente.
- Inactividad (umbral por defecto: **2 minutos**) → la sesión se marca como "idle".
- Bloqueo y suspensión del equipo cierran la sesión activa (Windows).
- Historial y estadísticas por proceso; todo persiste en SQLite al instante.

### Soporte por plataforma

| Plataforma | Captura nativa |
|---|---|
| **Windows** | Completa (ventana activa + inactividad + bloqueo/suspensión, Win32) |
| **Linux · X11** (GNOME/KDE/otros) | Ventana activa vía EWMH (`_NET_ACTIVE_WINDOW`) + inactividad vía XScreenSaver |
| **Linux · Wayland/GNOME** | Ventana focada vía D-Bus (`org.gnome.Shell.Introspect`) + inactividad (`org.gnome.Mutter.IdleMonitor`) |
| **Linux · Wayland/Sway** | Ventana focada vía el IPC de sway (`get_tree`) |
| **Linux · Wayland/KDE Plasma 6** | Ventana focada vía KWin Scripting (protocolo de kdotool) — *pendiente de validación en vivo* |
| **macOS** | Sin captura nativa aún (los instaladores se generan igualmente) |

Notas consabidas:

- La **sesión activa al cerrar la app** no se persiste (solo las cerradas); candidato de mejora futura.
- La inactividad en Wayland no-GNOME (Sway/KDE) aún no se detecta.
- En entornos sin backend nativo la app funciona normal, simplemente sin datos.
- La persistencia queda inactiva silenciosamente fuera de Tauri (p. ej. `next dev` en navegador).

## Requisitos

- **Node.js ≥ 20** + npm.
- **Rust estable** (https://rustup.rs).
- **Linux:** dependencias de sistema para webkit/GTK (las mismas que usa el CI):

  ```bash
  sudo apt install -y libwebkit2gtk-4.1-dev librsvg2-dev patchelf rpm build-essential libssl-dev libxdo-dev libappindicator3-dev
  ```

## Comandos

| Comando | Acción |
|---|---|
| `npm install` / `npm ci` | Instalar dependencias del frontend |
| `npm run dev` | Frontend en modo desarrollo → http://localhost:3100 |
| `npm run tauri dev` | App de escritorio en modo desarrollo (frontend + Rust) |
| `npm run build` | Build de producción del frontend (genera `out/`) |
| `npm run tauri build` | Build de producción **+ instaladores** |
| `npm run tauri` | CLI de Tauri |
| `npm run typecheck` | Comprobación de tipos (tsc) |
| `npm run lint` | ESLint (ver nota de deuda conocida) |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Tests del motor nativo (Rust) |

### Instaladores generados

Tras `npm run tauri build`, quedan en `src-tauri/target/release/bundle/`:

- **Windows:** `msi/` (WiX) y `nsis/` (`.exe`).
- **Linux:** `deb/`, `rpm/` y `appimage/`.
- **macOS:** `dmg/` y `app/` (Intel y Apple Silicon).

> Los instaladores **no están firmados**: Windows mostrará SmartScreen y macOS
> el aviso de Gatekeeper (clic derecho → Abrir la primera vez).

## Releases e instaladores (CI)

El workflow `.github/workflows/build-installers.yml`:

1. **Cada PR** → validación: build del frontend + `cargo test` en Ubuntu (sin generar instaladores).
2. **Manual** (`Actions` → *Run workflow*) o al **publicar una tag `v*`** → genera los instaladores de Windows/Linux y crea un **draft release** con tag **`app-monitor-vX.Y.Z`** (p. ej. `app-monitor-v0.1.2`).
3. **macOS** → opcional marcando el input **`build-macos`** (los runners tienen coste en repos privados; en repos públicos son gratis).
4. El draft se **publica manualmente** (botón *Publish release* o `gh release edit <tag> --draft=false`); al publicarse, GitHub lo marca automáticamente como *Latest* y lo hace descargable para todos.

La **versión** vive en `package.json`, `src-tauri/tauri.conf.json` y `src-tauri/Cargo.toml` — deben mantenerse en sync (bump a **v0.1.x** en los tres más los lockfiles).

## Guardado local (base de datos)

SQLite embebido vía `tauri-plugin-sql` + Drizzle ORM (adaptador `sqlite-proxy`):

- Fichero: `app-monitor.db` en el directorio de datos de la app
  (`%APPDATA%\com.appmonitor.desktop\` en Windows; `~/.local/share/com.appmonitor.desktop/` en Linux).
- Esquema: tabla `sessions` con índice único `(process_name, started_at, ended_at)` para evitar duplicados — definido en `lib/db/schema.ts`.
- Capa de datos: `lib/db/client.ts`, `lib/db/repository.ts`, `lib/db/stats.ts`.

## Estructura del proyecto

```
app/              UI por secciones (dashboard, historial, aplicaciones)
components/       componentes UI (dashboard, charts)
lib/db/           persistencia (schema, client, repository, stats)
src-tauri/        agente nativo Rust (Tauri 2)
  src/lib.rs      comandos Tauri + bucle de muestreo
  src/monitor.rs  motor de sesiones (independiente de plataforma)
  src/windows.rs  backend Win32 (ventana activa, inactividad)
  src/power.rs    bloqueo/suspensión (Win32)
  src/linux.rs    backend Linux (X11 + Wayland GNOME/Sway/KDE)
```

## Deuda conocida

- `npm run lint`: conflicto de peer de `@typescript-eslint` con `typescript@7` (solo afecta al lint, no al build).
- Instaladores sin firmar (requiere certificados de pago para eliminarlo).
- Backend KDE Plasma (Wayland): validado en compilación y tests, pendiente de confirmación en vivo sobre un escritorio KDE real.
- macOS: sin captura nativa de ventana activa (pendiente).