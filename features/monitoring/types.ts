export type ActivityState = "active" | "idle" | "locked" | "suspended";

export interface SessionView {
  window_title: string;
  process_name: string;
  application_name: string;
  started_at: number;
  ended_at: number;
  duration_ms: number;
  is_idle: boolean;
}

export type MonitoringEvent =
  | { session_start: SessionView }
  | { session_end: SessionView }
  | { activity_change: { state: ActivityState } };

export interface AgentStatus {
  ready: boolean;
  platform: string;
  version: string;
  monitoring: boolean;
}

export interface AppStatsView {
  application_name: string;
  process_name: string;
  total_duration_ms: number;
  session_count: number;
  first_seen: number;
  last_seen: number;
}
