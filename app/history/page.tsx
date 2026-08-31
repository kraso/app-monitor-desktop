"use client";

import { useState } from "react";
import { Card, CardHeader } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ActivityPanel } from "@/components/monitoring/activity-panel";
import { useMonitoring } from "@/app/use-monitoring";
import { formatDuration } from "@/lib/utils/format";

function formatDate(unixMillis: number): string {
  return new Date(unixMillis).toLocaleString("es-ES", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export default function History() {
  const { sessions } = useMonitoring();
  const [filter, setFilter] = useState<"all" | "active" | "idle">("all");

  const filtered = sessions.filter((s) => {
    if (filter === "active") return !s.is_idle;
    if (filter === "idle") return s.is_idle;
    return true;
  });

  return (
    <div className="flex flex-1 flex-col px-6 py-12">
      <header className="text-center">
        <h1 className="text-4xl font-bold tracking-tight text-slate-900 dark:text-white">
          Historial
        </h1>
        <p className="mt-2 text-slate-500 dark:text-slate-400">
          Consulta y filtra todas las sesiones de uso
        </p>
      </header>

      <ActivityPanel />

      <div className="mx-auto mt-6 w-full max-w-5xl animate-fade-in">
        <Card>
          <CardHeader
            title="Sesiones registradas"
            description={`${filtered.length} sesiones encontradas`}
            action={
              <div className="flex gap-2">
                <Button
                  variant={filter === "all" ? "primary" : "secondary"}
                  size="sm"
                  onClick={() => setFilter("all")}
                >
                  Todas
                </Button>
                <Button
                  variant={filter === "active" ? "primary" : "secondary"}
                  size="sm"
                  onClick={() => setFilter("active")}
                >
                  Activas
                </Button>
                <Button
                  variant={filter === "idle" ? "primary" : "secondary"}
                  size="sm"
                  onClick={() => setFilter("idle")}
                >
                  Inactivas
                </Button>
              </div>
            }
          />
          {filtered.length === 0 ? (
            <p className="py-8 text-center text-slate-400 dark:text-slate-500">
              No hay sesiones que coincidan con el filtro seleccionado.
            </p>
          ) : (
            <ul className="max-h-96 space-y-2 overflow-y-auto">
              {filtered.slice().reverse().map((s, i) => (
                <li
                  key={`${s.started_at}-${i}`}
                  className="flex items-center justify-between gap-4 rounded-xl bg-slate-50 px-4 py-3 dark:bg-slate-900/50"
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <p className="truncate font-semibold text-slate-800 dark:text-slate-200">
                        {s.application_name}
                      </p>
                      {s.is_idle && (
                        <span className="shrink-0 rounded-full bg-amber-100 px-2 py-0.5 text-xs font-semibold text-amber-700 dark:bg-amber-900/50 dark:text-amber-400">
                          Inactiva
                        </span>
                      )}
                    </div>
                    <p className="truncate text-sm text-slate-500 dark:text-slate-400">
                      {s.window_title}
                    </p>
                    <p className="mt-1 text-xs text-slate-400 dark:text-slate-500">
                      📅 {formatDate(s.started_at)} → {s.ended_at > 0 ? formatDate(s.ended_at) : "activa"}
                      <span className="mx-2">·</span>
                      <span className="font-mono">{s.process_name}</span>
                    </p>
                  </div>
                  <span className="shrink-0 rounded-lg bg-slate-200 px-3 py-1 text-sm font-bold text-slate-700 dark:bg-slate-700 dark:text-slate-300">
                    {formatDuration(s.duration_ms)}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </Card>
      </div>
    </div>
  );
}
