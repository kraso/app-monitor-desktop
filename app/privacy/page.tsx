"use client";

export default function Privacy() {
  return (
    <div className="flex flex-1 flex-col px-6 py-12 animate-fade-in">
      <header className="text-center">
        <h1 className="text-4xl font-bold tracking-tight text-slate-900 dark:text-white">
          Política de privacidad
        </h1>
        <p className="mt-2 text-lg text-slate-600 dark:text-slate-400">
          Transparencia sobre los datos que recogemos y cómo se usan.
        </p>
      </header>

      <div className="mx-auto mt-8 max-w-3xl space-y-6">
        <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-700 dark:bg-slate-800">
          <h2 className="text-xl font-semibold text-slate-900 dark:text-white">
            Datos recogidos
          </h2>
          <ul className="mt-3 list-disc space-y-1 pl-5 text-sm text-slate-600 dark:text-slate-400">
            <li>Título de la ventana en primer plano.</li>
            <li>Nombre del proceso asociado a la ventana.</li>
            <li>
              Instante de inicio y duración de cada sesión de uso.
            </li>
            <li>
              Estado de actividad del sistema (activo, inactivo, bloqueado,
              suspendido).
            </li>
          </ul>
        </section>

        <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-700 dark:bg-slate-800">
          <h2 className="text-xl font-semibold text-slate-900 dark:text-white">
            Finalidad del tratamiento
          </h2>
          <p className="mt-3 text-sm text-slate-600 dark:text-slate-400">
            Los datos se utilizan exclusivamente para generar estadísticas
            de productividad y analizar el uso de aplicaciones en tu equipo. No se
            envían a servidores externos: todo el procesamiento es local.
          </p>
        </section>

        <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-700 dark:bg-slate-800">
          <h2 className="text-xl font-semibold text-slate-900 dark:text-white">
            Almacenamiento
          </h2>
          <p className="mt-3 text-sm text-slate-600 dark:text-slate-400">
            Las sesiones se mantienen en memoria mientras el monitor está
            activo. Al detenerlo o cerrar la aplicación, el historial se
            vacía. No utilizamos almacenamiento persistente en disco para
            los datos de monitorización.
          </p>
        </section>

        <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm dark:border-slate-700 dark:bg-slate-800">
          <h2 className="text-xl font-semibold text-slate-900 dark:text-white">Tus derechos</h2>
          <p className="mt-3 text-sm text-slate-600 dark:text-slate-400">
            Puedes detener la monitorización en cualquier momento con un
            solo clic. Si tienes dudas sobre la privacidad de la aplicación,
            revisa el código fuente del agente nativo en el directorio{" "}
            <code className="rounded bg-slate-100 px-1.5 py-0.5 text-xs dark:bg-slate-700">
              src-tauri/src
            </code>
            .
          </p>
        </section>
      </div>
    </div>
  );
}
