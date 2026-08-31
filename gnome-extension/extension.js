/* App Monitor - Window Snapshot
 *
 * Expone la ventana activa (título, clase WM, pid) de GNOME Shell a través de
 * D-Bus. Es la forma que mantienen GNOME 46+ para que apps de terceros
 * conozcan la ventana focada NATIVA de Wayland (la política del Shell bloquea
 * Introspect.GetWindows y Shell.Eval).
 *
 * El objeto D-Bus se exporta en la conexión de sesión de GNOME Shell; la app
 * lo consume contra el nombre bien conocido "org.gnome.Shell" con el
 * object path de esta extensión (patrón usado por otras extensiones). NO se
 * adquiere un nombre de bus propio: la API Gio.bus_own_name_on_session se
 * eliminó de GJS (GNOME 46+) y este enfoque es el estable.
 *
 * Instalación: copia esta carpeta a
 *   ~/.local/share/gnome-shell/extensions/app-monitor@kraso.dev/
 * y activa la extensión (reinicia la sesión o abre la app "Extensiones" y
 * enciéndela).
 */

import Gio from 'gi://Gio';

const OBJECT_PATH = '/com/appmonitor/desktop/WindowInfo';

const IFACE_XML = `
<node>
  <interface name="com.appmonitor.desktop.WindowInfo">
    <method name="GetActiveWindow">
      <arg type="s" name="title" direction="out" />
      <arg type="s" name="wm_class" direction="out" />
      <arg type="u" name="pid" direction="out" />
    </method>
  </interface>
</node>`;

export default class AppMonitorWindowInfoExtension {
    enable() {
        this._dbusImpl = Gio.DBusExportedObject.wrapJSObject(IFACE_XML, this);
        this._dbusImpl.export(Gio.DBus.session, OBJECT_PATH);
    }

    disable() {
        if (this._dbusImpl) {
            this._dbusImpl.unexport();
            this._dbusImpl = null;
        }
    }

    GetActiveWindow() {
        const win = global.display.focus_window;
        if (!win) {
            return ['', '', 0];
        }
        let wmClass = '';
        try {
            wmClass = win.get_wm_class() || '';
        } catch (e) {
            wmClass = '';
        }
        const title = win.get_title() || '';
        // get_pid() puede devolver 0 en clientes Wayland; la app resuelve el
        // proceso por el pid y, si no, cae a la clase WM.
        const pid = Number.isFinite(win.get_pid()) ? win.get_pid() : 0;
        return [title, wmClass, pid];
    }
}