"use client";

import { useState, useEffect } from "react";
import { Card, CardHeader } from "@/components/ui/card";
import { ActivityPanel } from "@/components/monitoring/activity-panel";
import { useMonitoring } from "@/app/use-monitoring";
import { formatDuration } from "@/lib/utils/format";

export default function Dashboard() {
  const { monitoring, sessions, appStats, refresh } = useMonitoring();
  const [, setTick] = useState(0);

  useEffect(() => {
    if (!monitoring) return;
    const id = setInterval(() => {
      setTick((t) => t + 1);
      refresh().catch(() => {});
    }, 3000);
    return () => clearInterval(id);
  }, [monitoring, refresh]);

  const totalMs = sessions.reduce((acc, s) => acc + (s.duration_ms || 0), 0);
  const activeSessions = sessions.filter((s) => !s.is_idle);
  const idleSessions = sessions.filter((s) => s.is_idle);
  const topApp = appStats.length > 0 ? appStats[0] : null;

  return (
    <div className="flex flex-1 flex-col items-center gap-8 animate-fade-in">
      <header className="text-center">
        <h1 className="text-4xl font-bold tracking-tight text-slate-900 dark:text-white">
          Dashboard
        </h1>
        <p className="mt-2 text-slate-500 dark:text-slate-400">
          Estadísticas en tiempo real de tus aplicaciones
        </p>
      </header>

      <ActivityPanel />

      <div className="grid w-full max-w-5xl grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
        <Card className="p-6 animate-scale-in">
          <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-blue-100 text-blue-600 dark:bg-blue-900/50 dark:text-blue-400">
            ⏱️
          </div>
          <CardHeader title="Tiempo total" description="Acumulado en esta sesión" />
          <p className="text-3xl font-bold text-slate-900 dark:text-white">
            {monitoring ? formatDuration(totalMs) : "—"}
          </p>
          {monitoring && (
            <p className="mt-2 text-xs text-slate-400 dark:text-slate-500">
              {sessions.length} sesiones registradas
            </p>
          )}
        </Card>

        <Card className="p-6 animate-scale-in" style={{ animationDelay: "0.1s" }}>
          <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-green-100 text-green-600 dark:bg-green-900/50 dark:text-green-400">
            🎯
          </div>
          <CardHeader title="Sesiones activas" description="Sesiones sin inactividad" />
          <p className="text-3xl font-bold text-slate-900 dark:text-white">
            {monitoring ? activeSessions.length : "—"}
          </p>
          {monitoring && (
            <p className="mt-2 text-xs text-slate-400 dark:text-slate-500">
              {idleSessions.length} en inactividad
            </p>
          )}
        </Card>

        <Card className="p-6 animate-scale-in" style={{ animationDelay: "0.2s" }}>
          <div className="mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-amber-100 text-amber-600 dark:bg-amber-900/50 dark:text-amber-400">
            🏆
          </div>
          <CardHeader title="Top app" description="App con más uso acumulado" />
          <p className="text-lg font-bold text-slate-900 dark:text-white">
            {monitoring && topApp ? topApp.application_name : "—"}
          </p>
          {monitoring && topApp && (
            <p className="mt-2 text-xs text-slate-400 dark:text-slate-500">
              {formatDuration(topApp.total_duration_ms)} · {topApp.session_count} sesiones
            </p>
          )}
        </Card>
      </div>
    </div>
  );
}
