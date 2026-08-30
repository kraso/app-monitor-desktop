"use client";

import { useState, useMemo, useEffect } from "react";
import { Card, CardHeader } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ActivityPanel } from "@/components/monitoring/activity-panel";
import { useMonitoring } from "@/app/use-monitoring";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { formatDuration } from "@/lib/utils/format";

function formatDate(unixMillis: number): string {
  return new Date(unixMillis).toLocaleString("es-ES", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

type ExportFormat = "csv" | "html" | "json";
type DateRange = "all" | "today" | "week" | "month";

export default function Reports() {
  const { sessions } = useMonitoring();
  const [exporting, setExporting] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [dateRange, setDateRange] = useState<DateRange>("all");
  const [format, setFormat] = useState<ExportFormat>("csv");
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const interval = setInterval(() => setNow(Date.now()), 60_000);
    return () => clearInterval(interval);
  }, []);

  const showMessage = (msg: string) => {
    setMessage(msg);
    setTimeout(() => setMessage(null), 5000);
  };

  const filteredSessions = useMemo(() => {
    if (dateRange === "all") return sessions;
    const dayMs = 24 * 60 * 60 * 1000;
    let cutoff = 0;
    if (dateRange === "today") cutoff = now - dayMs;
    else if (dateRange === "week") cutoff = now - 7 * dayMs;
    else if (dateRange === "month") cutoff = now - 30 * dayMs;
    return sessions.filter((s) => s.started_at >= cutoff);
  }, [sessions, dateRange, now]);

  const totalMs = filteredSessions.reduce((acc, s) => acc + s.duration_ms, 0);
  const activeSessions = filteredSessions.filter((s) => !s.is_idle);
  const idleSessions = filteredSessions.filter((s) => s.is_idle);

  const handleExport = async () => {
    if (filteredSessions.length === 0) {
      showMessage("No hay datos para exportar en el rango seleccionado.");
      return;
    }
    setExporting(true);
    try {
      const defaultName = `app-monitor-${dateRange}`;
      let filePath: string | null = null;
      let content = "";

      if (format === "csv") {
        filePath = await save({
          defaultPath: `${defaultName}.csv`,
          filters: [{ name: "CSV", extensions: ["csv"] }],
        });
        if (filePath) {
          const headers = [
            "application_name",
            "process_name",
            "window_title",
            "started_at",
            "duration",
            "is_idle",
          ];
          const rows = filteredSessions.map((s) => {
            const d = new Date(s.started_at);
            const dd = String(d.getDate()).padStart(2, "0");
            const mm = String(d.getMonth() + 1).padStart(2, "0");
            const yyyy = d.getFullYear();
            const hh = String(d.getHours()).padStart(2, "0");
            const min = String(d.getMinutes()).padStart(2, "0");
            const ss = String(d.getSeconds()).padStart(2, "0");
            const startedAt = `${dd}/${mm}/${yyyy} ${hh}:${min}:${ss}`;
            return [
              s.application_name,
              s.process_name,
              `"${s.window_title.replace(/"/g, '""')}"`,
              startedAt,
              formatDuration(s.duration_ms),
              s.is_idle,
            ].join(",");
          });
          content = [headers.join(","), ...rows].join("\n");
          await writeTextFile(filePath, content);
          showMessage("CSV guardado correctamente");
        }
      } else if (format === "json") {
        filePath = await save({
          defaultPath: `${defaultName}.json`,
          filters: [{ name: "JSON", extensions: ["json"] }],
        });
        if (filePath) {
          const payload = {
            generated_at: new Date().toISOString(),
            total_sessions: filteredSessions.length,
            total_duration_ms: filteredSessions.reduce((a, s) => a + s.duration_ms, 0),
            sessions: filteredSessions.map((s) => ({
              ...s,
              started_at_iso: new Date(s.started_at).toISOString(),
              formatted_duration: formatDuration(s.duration_ms),
              formatted_date: formatDate(s.started_at),
            })),
          };
          content = JSON.stringify(payload, null, 2);
          await writeTextFile(filePath, content);
          showMessage("JSON guardado correctamente");
        }
      } else {
        filePath = await save({
          defaultPath: `${defaultName}.html`,
          filters: [{ name: "HTML", extensions: ["html"] }],
        });
        if (filePath) {
          content = `<!DOCTYPE html><html lang="es"><head><meta charset="utf-8"><title>Informe</title></head><body style="font-family:system-ui;padding:2rem"><h1>Informe de productividad</h1><p>Generado: ${new Date().toLocaleString("es-ES")}</p><table border="1" cellpadding="8" style="border-collapse:collapse;width:100%"><tr><th style="text-align:center">App</th><th style="text-align:center">Duración</th></tr>${filteredSessions.map((s) => `<tr><td>${s.application_name}</td><td style="text-align:right">${formatDuration(s.duration_ms)}</td></tr>`).join("")}</table></body></html>`;
          await writeTextFile(filePath, content);
          showMessage("HTML guardado. Ábrelo para imprimir.");
        }
      }
      if (!filePath) {
        showMessage("Exportación cancelada por el usuario");
      }
    } catch (err) {
      console.error("Error al exportar:", err);
      showMessage(`Error: ${err}`);
    } finally {
      setExporting(false);
    }
  };

  return (
    <div className="flex flex-1 flex-col px-6 py-12">
      <header className="text-center">
        <h1 className="text-4xl font-bold tracking-tight text-slate-900 dark:text-white">
          Informes
        </h1>
        <p className="mt-2 text-slate-500 dark:text-slate-400">
          Descarga informes detallados de tu actividad
        </p>
      </header>

      <ActivityPanel />

      <div className="mx-auto mt-6 w-full max-w-5xl space-y-6 animate-fade-in">
        <Card>
          <CardHeader title="Resumen" description="Estadísticas del rango seleccionado" />
          <div className="mt-4 grid grid-cols-2 gap-4 sm:grid-cols-4">
            <div className="rounded-xl bg-slate-50 p-4 dark:bg-slate-900/50">
              <p className="text-2xl font-bold text-slate-900 dark:text-white">{filteredSessions.length}</p>
              <p className="text-sm text-slate-500 dark:text-slate-400">Sesiones</p>
            </div>
            <div className="rounded-xl bg-slate-50 p-4 dark:bg-slate-900/50">
              <p className="text-2xl font-bold text-slate-900 dark:text-white">{formatDuration(totalMs)}</p>
              <p className="text-sm text-slate-500 dark:text-slate-400">Tiempo total</p>
            </div>
            <div className="rounded-xl bg-slate-50 p-4 dark:bg-slate-900/50">
              <p className="text-2xl font-bold text-slate-900 dark:text-white">{activeSessions.length}</p>
              <p className="text-sm text-slate-500 dark:text-slate-400">Activas</p>
            </div>
            <div className="rounded-xl bg-slate-50 p-4 dark:bg-slate-900/50">
              <p className="text-2xl font-bold text-slate-900 dark:text-white">{idleSessions.length}</p>
              <p className="text-sm text-slate-500 dark:text-slate-400">Inactivas</p>
            </div>
          </div>
        </Card>

        <Card>
          <CardHeader title="Exportar informe" description="Selecciona rango y formato" />
          <div className="mt-4 space-y-4">
            <div>
              <label className="text-sm font-semibold text-slate-700 dark:text-slate-300">Rango de fechas</label>
              <div className="mt-2 flex flex-wrap gap-2">
                <Button variant={dateRange === "all" ? "primary" : "secondary"} size="sm" onClick={() => setDateRange("all")}>Todo</Button>
                <Button variant={dateRange === "today" ? "primary" : "secondary"} size="sm" onClick={() => setDateRange("today")}>Hoy</Button>
                <Button variant={dateRange === "week" ? "primary" : "secondary"} size="sm" onClick={() => setDateRange("week")}>Semana</Button>
                <Button variant={dateRange === "month" ? "primary" : "secondary"} size="sm" onClick={() => setDateRange("month")}>Mes</Button>
              </div>
            </div>

            <div>
              <label className="text-sm font-semibold text-slate-700 dark:text-slate-300">Formato</label>
              <div className="mt-2 flex flex-wrap gap-2">
                <Button variant={format === "csv" ? "primary" : "secondary"} size="sm" onClick={() => setFormat("csv")}>CSV</Button>
                <Button variant={format === "html" ? "primary" : "secondary"} size="sm" onClick={() => setFormat("html")}>HTML</Button>
                <Button variant={format === "json" ? "primary" : "secondary"} size="sm" onClick={() => setFormat("json")}>JSON</Button>
              </div>
            </div>

            <div className="flex items-center gap-4 pt-2">
              <Button onClick={handleExport} disabled={exporting || filteredSessions.length === 0} className="min-w-[160px]">
                {exporting ? "Exportando..." : "Descargar"}
              </Button>
              <span className="text-sm text-slate-400 dark:text-slate-500">{filteredSessions.length} sesiones</span>
            </div>

            {message && (
              <div className="rounded-xl bg-blue-50 p-3 text-sm text-blue-700 dark:bg-blue-950 dark:text-blue-300">{message}</div>
            )}
          </div>
        </Card>
      </div>
    </div>
  );
}
