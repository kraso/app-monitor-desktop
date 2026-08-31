"use client";

import { useMonitoring } from "@/app/use-monitoring";
import type { ActivityState, SessionView } from "@/features/monitoring/types";
import { Button } from "@/components/ui/button";
import { Card, CardHeader } from "@/components/ui/card";
import { formatDuration } from "@/lib/utils/format";

export function ActivityPanel() {
  const { monitoring, active, state, sessions, agentStatus, start, stop } =
    useMonitoring();

  const isNative = agentStatus !== null;

  return (
    <section className="w-full animate-fade-in">
      <Card>
        <CardHeader
          title="Monitorización en tiempo real"
          description={monitoring ? "Capturando actividad del sistema" : "Inicia la monitorización para comenzar"}
          action={
            <div className="flex items-center gap-3">
              {isNative && (
                <span className={`flex items-center gap-2 rounded-full px-3 py-1.5 text-xs font-semibold ${
                  monitoring
                    ? "bg-green-100 text-green-700 dark:bg-green-900/50 dark:text-green-400"
                    : "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400"
                }`}>
                  <span className={`h-2 w-2 rounded-full ${monitoring ? "animate-pulse-soft bg-green-500" : "bg-slate-400"}`} />
                  {monitoring ? "Activo" : "Detenido"}
                </span>
              )}
              {monitoring ? (
                <Button variant="danger" size="sm" onClick={stop}>
                  Detener
                </Button>
              ) : (
                <Button size="sm" onClick={start} disabled={!isNative}>
                  Iniciar
                </Button>
              )}
            </div>
          }
        />

        {!isNative && (
          <div className="rounded-xl bg-amber-50 p-4 text-sm text-amber-800 dark:bg-amber-950/50 dark:text-amber-300">
            ⚠️ Ejecuta <code className="rounded bg-amber-100 px-1.5 py-0.5 font-mono text-xs dark:bg-amber-900">npm run tauri dev</code> para activar la monitorización nativa.
          </div>
        )}

        {isNative && (
          <div className="grid gap-4 md:grid-cols-2">
            <ActiveCard active={active} state={state} />
            <HistoryList sessions={sessions} />
          </div>
        )}
      </Card>
    </section>
  );
}

const stateConfig: Record<ActivityState, { label: string; color: string }> = {
  active: { label: "Activo", color: "bg-green-100 text-green-700 dark:bg-green-900/50 dark:text-green-400" },
  idle: { label: "Inactivo", color: "bg-amber-100 text-amber-700 dark:bg-amber-900/50 dark:text-amber-400" },
  locked: { label: "Bloqueado", color: "bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-300" },
  suspended: { label: "Suspendido", color: "bg-slate-100 text-slate-700 dark:bg-slate-800 dark:text-slate-300" },
};

function ActiveCard({ active, state }: { active: SessionView | null; state: ActivityState }) {
  const { label, color } = stateConfig[state];

  return (
    <div className="rounded-xl bg-slate-50 p-4 dark:bg-slate-900/50">
      <div className="mb-3 flex items-center justify-between">
        <h4 className="text-sm font-semibold text-slate-700 dark:text-slate-300">Sesión actual</h4>
        <span className={`rounded-full px-2.5 py-1 text-xs font-semibold ${color}`}>{label}</span>
      </div>
      {active ? (
        <div className="space-y-1">
          <p className="text-lg font-bold text-slate-900 dark:text-white">{active.application_name}</p>
          <p className="truncate text-sm text-slate-500 dark:text-slate-400">{active.window_title}</p>
          <p className="font-mono text-xs text-slate-400 dark:text-slate-500">{active.process_name}</p>
        </div>
      ) : (
        <p className="text-sm text-slate-400 dark:text-slate-500">Sin sesión activa</p>
      )}
    </div>
  );
}

function HistoryList({ sessions }: { sessions: SessionView[] }) {
  return (
    <div className="rounded-xl bg-slate-50 p-4 dark:bg-slate-900/50">
      <h4 className="mb-3 text-sm font-semibold text-slate-700 dark:text-slate-300">Historial reciente</h4>
      {sessions.length === 0 ? (
        <p className="text-sm text-slate-400 dark:text-slate-500">Aún no hay sesiones registradas.</p>
      ) : (
        <ul className="max-h-48 space-y-2 overflow-y-auto">
          {sessions.slice().reverse().slice(0, 5).map((s, i) => (
            <li key={`${s.started_at}-${i}`} className="flex items-center justify-between rounded-lg bg-white px-3 py-2 shadow-sm dark:bg-slate-800">
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium text-slate-800 dark:text-slate-200">{s.application_name}</p>
                <p className="truncate text-xs text-slate-400 dark:text-slate-500">{s.window_title}</p>
              </div>
              <span className="shrink-0 text-xs font-semibold text-slate-500 dark:text-slate-400">
                {formatDuration(s.duration_ms)}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}


