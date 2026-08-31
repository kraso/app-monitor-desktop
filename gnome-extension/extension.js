/* App Monitor - Window Snapshot
 *
 * Expone la ventana activa (título, clase WM, pid) de GNOME Shell a través de
 * un servicio D-Bus de sesión propio. Es la forma que mantiene GNOME 46+ para
 * que apps de terceros conozcan la ventana focada NATIVA de Wayland (la
 * política del Shell bloquea Introspect.GetWindows y Shell.Eval).
 *
 * Instalación: copia esta carpeta a
 *   ~/.local/share/gnome-shell/extensions/app-monitor@kraso.dev/
 * y activa la extensión (reinicia la sesión o abre la app "Extensiones" y
 * enciéndela).
 */

import Gio from 'gi://Gio';

const BUS_NAME = 'com.appmonitor.desktop.WindowInfo';
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
        this._busNameId = Gio.bus_own_name_on_session(
            BUS_NAME,
            Gio.BusNameOwnerFlags.ALLOW_REPLACEMENT,
            () => {},
            () => {},
            () => console.error('app-monitor: no se pudo adquirir el nombre D-Bus ' + BUS_NAME),
        );
    }

    disable() {
        if (this._busNameId) {
            Gio.bus_unown_name(this._busNameId);
            this._busNameId = 0;
        }
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