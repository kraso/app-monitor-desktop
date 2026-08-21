"use client";

import { useEffect } from "react";
import { Card, CardHeader } from "@/components/ui/card";
import { ActivityPanel } from "@/components/monitoring/activity-panel";
import { useMonitoring } from "@/app/use-monitoring";
import type { AppStatsView } from "@/features/monitoring/types";

function formatDuration(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const h = Math.floor(totalSeconds / 3600);
  const m = Math.floor((totalSeconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatDate(unixMillis: number): string {
  return new Date(unixMillis).toLocaleString("es-ES", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

const COLORS = [
  "from-blue-500 to-blue-600",
  "from-green-500 to-green-600",
  "from-amber-500 to-amber-600",
  "from-red-500 to-red-600",
  "from-purple-500 to-purple-600",
  "from-cyan-500 to-cyan-600",
  "from-pink-500 to-pink-600",
  "from-teal-500 to-teal-600",
];

function AppStatCard({ app, maxMs, color, index }: { app: AppStatsView; maxMs: number; color: string; index: number }) {
  const pct = maxMs > 0 ? Math.max(2, Math.round((app.total_duration_ms / maxMs) * 100)) : 0;
  const avgMs = app.session_count > 0 ? Math.round(app.total_duration_ms / app.session_count) : 0;

  return (
    <li className="rounded-xl bg-slate-50 p-4 transition-all hover:bg-slate-100 dark:bg-slate-900/50 dark:hover:bg-slate-800/50" style={{ animationDelay: `${index * 0.05}s` }}>
      <div className="flex items-center justify-between">
        <div className="flex items-center space-x-3">
          <span className={`h-3 w-3 rounded-full bg-gradient-to-r ${color}`} />
          <span className="font-semibold text-slate-800 dark:text-slate-200">{app.application_name}</span>
          <span className="rounded-full bg-slate-200 px-2 py-0.5 text-xs font-semibold text-slate-600 dark:bg-slate-700 dark:text-slate-300">
            {app.session_count} sesiones
          </span>
        </div>
        <span className="text-lg font-bold text-slate-900 dark:text-white">{formatDuration(app.total_duration_ms)}</span>
      </div>
      <div className="mt-3 h-2 rounded-full bg-slate-200 dark:bg-slate-700">
        <div className={`h-2 rounded-full bg-gradient-to-r ${color} progress-animate`} style={{ width: `${pct}%` }} />
      </div>
      <div className="mt-2 flex gap-4 text-xs text-slate-500 dark:text-slate-400">
        <span>Primera: {formatDate(app.first_seen)}</span>
        <span>Ultima: {formatDate(app.last_seen)}</span>
        <span>Media: {formatDuration(avgMs)}</span>
      </div>
    </li>
  );
}

export default function Applications() {
  const { appStats, monitoring, refresh } = useMonitoring();
  const maxMs = appStats.length > 0 ? appStats[0]?.total_duration_ms ?? 1 : 1;
  const totalMs = appStats.reduce((acc, s) => acc + s.total_duration_ms, 0);
  const totalSessions = appStats.reduce((acc, s) => acc + s.session_count, 0);

  // Refrescar datos periodicamente mientras el monitor esta activo
  useEffect(() => {
    if (!monitoring) return;
    const interval = setInterval(() => {
      refresh();
    }, 3000);
    return () => clearInterval(interval);
  }, [monitoring, refresh]);

  return (
    <div className="flex flex-1 flex-col px-6 py-12">
      <header className="text-center">
        <h1 className="text-4xl font-bold tracking-tight text-slate-900 dark:text-white">
          Aplicaciones
        </h1>
        <p className="mt-2 text-slate-500 dark:text-slate-400">
          Tiempo total de uso por aplicacion
        </p>
      </header>

      <ActivityPanel />

      <div className="mx-auto mt-6 w-full max-w-5xl space-y-6 animate-fade-in">
        <Card>
          <CardHeader title="Resumen global" description="Estadisticas de uso total" />
          <div className="mt-4 grid grid-cols-3 gap-6">
            <div className="rounded-xl bg-gradient-to-br from-blue-50 to-blue-100 p-4 dark:from-blue-950/50 dark:to-blue-900/50">
              <p className="text-3xl font-bold text-blue-600 dark:text-blue-400">{formatDuration(totalMs)}</p>
              <p className="mt-1 text-sm text-blue-600/70 dark:text-blue-300/70">Tiempo total</p>
            </div>
            <div className="rounded-xl bg-gradient-to-br from-green-50 to-green-100 p-4 dark:from-green-950/50 dark:to-green-900/50">
              <p className="text-3xl font-bold text-green-600 dark:text-green-400">{appStats.length}</p>
              <p className="mt-1 text-sm text-green-600/70 dark:text-green-300/70">Aplicaciones</p>
            </div>
            <div className="rounded-xl bg-gradient-to-br from-purple-50 to-purple-100 p-4 dark:from-purple-950/50 dark:to-purple-900/50">
              <p className="text-3xl font-bold text-purple-600 dark:text-purple-400">{totalSessions}</p>
              <p className="mt-1 text-sm text-purple-600/70 dark:text-purple-300/70">Sesiones</p>
            </div>
          </div>
        </Card>

        <Card>
          <CardHeader title="Uso por aplicacion" description="Ordenado por tiempo total de uso" />
          {appStats.length === 0 ? (
            <p className="py-8 text-center text-slate-400 dark:text-slate-500">
              {monitoring
                ? "Sin datos aun. Usa algunas aplicaciones para generar estadisticas."
                : "Inicia la monitorizacion para ver las estadisticas."}
            </p>
          ) : (
            <ul className="space-y-3">
              {appStats.map((app, i) => (
                <AppStatCard
                  key={app.process_name}
                  app={app}
                  maxMs={maxMs}
                  color={COLORS[i % COLORS.length] ?? "from-blue-500 to-blue-600"}
                  index={i}
                />
              ))}
            </ul>
          )}
        </Card>
      </div>
    </div>
  );
}
