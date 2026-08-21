"use client";

import { useMonitoringContext } from "./monitoring-provider";

/**
 * Hook de compatibilidad. Internamente delega en el contexto global
 * de monitorización para que el estado se comparta entre todas las páginas.
 */
export function useMonitoring() {
  return useMonitoringContext();
}
