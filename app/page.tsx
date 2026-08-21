"use client";

import Link from "next/link";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { ActivityPanel } from "@/components/monitoring/activity-panel";
import { useMonitoring } from "@/app/use-monitoring";

const sections = [
  {
    href: "/dashboard",
    label: "Dashboard",
    icon: "📊",
    desc: "Estadísticas en tiempo real de tu productividad",
    gradient: "from-blue-500 to-blue-600",
  },
  {
    href: "/history",
    label: "Historial",
    icon: "📋",
    desc: "Consultas y filtros de sesiones",
    gradient: "from-green-500 to-green-600",
  },
  {
    href: "/applications",
    label: "Aplicaciones",
    icon: "💻",
    desc: "Tiempo total por aplicación",
    gradient: "from-purple-500 to-purple-600",
  },
  {
    href: "/reports",
    label: "Informes",
    icon: "📈",
    desc: "Exporta datos de productividad",
    gradient: "from-amber-500 to-amber-600",
  },
  {
    href: "/settings",
    label: "Ajustes",
    icon: "⚙️",
    desc: "Configuración y privacidad",
    gradient: "from-slate-500 to-slate-600",
  },
];

function AgentBadge() {
  const { agentStatus, agentLoading } = useMonitoring();

  if (agentLoading) {
    return (
      <span className="inline-flex items-center gap-2 rounded-full bg-slate-100 px-4 py-2 text-sm text-slate-600 dark:bg-slate-800 dark:text-slate-300">
        <span className="h-2 w-2 animate-pulse rounded-full bg-slate-400" />
        Conectando...
      </span>
    );
  }

  if (agentStatus?.ready) {
    return (
      <span className="inline-flex items-center gap-2 rounded-full bg-green-100 px-4 py-2 text-sm font-semibold text-green-700 dark:bg-green-900/50 dark:text-green-400">
        <span className="h-2 w-2 rounded-full bg-green-500 animate-pulse-soft" />
        Agente nativo activo · v{agentStatus.version}
      </span>
    );
  }

  return (
    <span className="inline-flex items-center gap-2 rounded-full bg-amber-100 px-4 py-2 text-sm font-semibold text-amber-700 dark:bg-amber-900/50 dark:text-amber-400">
      <span className="h-2 w-2 rounded-full bg-amber-500" />
      Modo web (sin agente nativo)
    </span>
  );
}

export default function Home() {
  return (
    <div className="flex flex-1 flex-col items-center gap-10 animate-fade-in">
      <header className="text-center">
        <div className="mb-4 inline-flex h-20 w-20 items-center justify-center rounded-2xl bg-gradient-to-br from-blue-500 to-blue-600 text-4xl font-bold text-white shadow-xl shadow-blue-500/30">
          A
        </div>
        <h1 className="text-4xl font-bold tracking-tight text-slate-900 dark:text-white">
          App Monitor
        </h1>
        <p className="mt-2 max-w-md text-lg text-slate-500 dark:text-slate-400">
          Monitoriza el uso de aplicaciones y analiza tu productividad
        </p>
        <div className="mt-4">
          <AgentBadge />
        </div>
      </header>

      <ActivityPanel />

      <div className="grid w-full max-w-5xl grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {sections.map((s) => (
          <Link key={s.href} href={s.href}>
            <Card hover className="h-full group">
              <div className={`mb-4 inline-flex h-12 w-12 items-center justify-center rounded-xl bg-gradient-to-br ${s.gradient} text-xl text-white shadow-lg transition-transform group-hover:scale-110`}>
                {s.icon}
              </div>
              <h3 className="text-lg font-semibold text-slate-900 dark:text-white">{s.label}</h3>
              <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">{s.desc}</p>
            </Card>
          </Link>
        ))}
      </div>

      <Button variant="secondary" size="lg">
        <Link href="/dashboard">Ir al Dashboard →</Link>
      </Button>
    </div>
  );
}
