# App Monitor — Extensión de GNOME Shell

Expone la **ventana activa** (título, clase WM, pid) de GNOME Shell a través de
un servicio D-Bus de sesión, para que **App Monitor** pueda monitorizar las
aplicaciones **nativas de Wayland**.

> ¿Por qué es necesaria? En GNOME 46+ (p. ej. Ubuntu 24.04) la política del
> Shell **bloquea deliberadamente** `org.gnome.Shell.Introspect.GetWindows` y
> `org.gnome.Shell.Eval` para apps de terceros (`AccessDenied`). La única vía
> que deja GNOME para conocer la ventana focada nativa es una **extensión**
> (es lo que hace ActivityWatch).

## Instalación (en el equipo con GNOME)

```bash
mkdir -p ~/.local/share/gnome-shell/extensions/app-monitor@kraso.dev
cp metadata.json extension.js ~/.local/share/gnome-shell/extensions/app-monitor@kraso.dev/
```

- **Sesión X11**: `Alt+F2` → `r` → Enter (reinicia el shell y carga la extensión).
- **Sesión Wayland**: cierra sesión y vuelve a entrar (o activa la extensión
  en la app *Extensiones*).

Comprueba que quedó cargada:

```bash
gnome-extensions info app-monitor@kraso.dev
```

## Verificación manual del servicio D-Bus

```bash
gdbus call --session \
  --dest org.gnome.Shell \
  --object-path /com/appmonitor/desktop/WindowInfo \
  --method com.appmonitor.desktop.WindowInfo.GetActiveWindow
```

Debería devolver algo como `('Terminal', 'org.gnome.Terminal', uint32 1234)`
(o `('', '', uint32 0)` si no hay ventana focada).

## Cómo lo usa App Monitor

El backend Linux de App Monitor, en sesión GNOME, consulta esta extensión por
D-Bus; si no está instalada o el Shell bloquea las APIs nativas, la app cae al
fallback X11/XWayland (solo ve aplicaciones X11) — por eso para una cobertura
completa de Wayland conviene tener la extensión instalada.

## Compatibilidad

- Probada con la API de extensiones de GNOME 45 → 50 (`Gio.DBusExportedObject`
  + `global.display.focus_window`).