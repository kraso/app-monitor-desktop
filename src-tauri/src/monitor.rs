//! Motor de monitorización — máquina de estados de sesiones.
//!
//! Fase 3: captura de ventana activa, proceso, inactividad, bloqueo y
//! suspensión. Este módulo es independiente de la plataforma y contiene
//! la lógica pura: mantiene la sesión activa actual, la cierra cuando
//! cambia el proceso en primer plano (o al bloquear/suspender) y acumula
//! el historial.

use serde::Serialize;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Estado de actividad del sistema en un momento dado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityState {
    /// El usuario está interactuando.
    Active,
    /// El sistema detectó inactividad.
    Idle,
    /// La sesión de Windows está bloqueada.
    Locked,
    /// El equipo está en suspensión.
    Suspended,
}

/// Una sesión de uso de una aplicación/ventana.
#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub window_title: String,
    pub process_name: String,
    pub application_name: String,
    /// Instante de inicio (unix millis).
    pub started_at: u64,
    /// Instante de finalización (unix millis). 0 si está activa.
    pub ended_at: u64,
    /// Duración total en milisegundos (calculada al cerrar).
    pub duration_ms: u64,
    /// Verdadero si la sesión transcurrió en estado de inactividad.
    pub is_idle: bool,
}

impl Session {
    pub fn new(
        window_title: String,
        process_name: String,
        application_name: String,
        started_at: u64,
        is_idle: bool,
    ) -> Self {
        Session {
            window_title,
            process_name,
            application_name,
            started_at,
            ended_at: 0,
            duration_ms: 0,
            is_idle,
        }
    }

    /// Calcula la duración a partir del instante de cierre.
    pub fn finalize(&mut self, ended_at: u64) {
        self.ended_at = ended_at;
        self.duration_ms = ended_at.saturating_sub(self.started_at);
    }
}

/// Estadísticas agregadas por aplicación.
#[derive(Debug, Clone, Serialize)]
pub struct AppStats {
    pub application_name: String,
    pub process_name: String,
    pub total_duration_ms: u64,
    pub session_count: u64,
    pub first_seen: u64,
    pub last_seen: u64,
}

/// Estado del monitor: estado de actividad, sesión activa y historial.
pub struct Monitor {
    state: ActivityState,
    active: Option<Session>,
    sessions: Vec<Session>,
    idle_threshold_ms: u64,
}

impl Monitor {
    pub fn new(idle_threshold_ms: u64) -> Self {
        Monitor {
            state: ActivityState::Active,
            active: None,
            sessions: Vec::new(),
            idle_threshold_ms,
        }
    }

    pub fn idle_threshold_ms(&self) -> u64 {
        self.idle_threshold_ms
    }

    /// Cambia el estado de actividad. Al bloquear o suspender, cierra la
    /// sesión activa (regla del plan: "cerrar sesiones al apagar").
    pub fn set_activity(&mut self, state: ActivityState, now: u64) {
        if matches!(state, ActivityState::Locked | ActivityState::Suspended) {
            self.close_active(now);
        }
        self.state = state;
    }

    /// Procesa un tick con la ventana en primer plano.
    /// Devuelve `Some(Session)` si se cerró una sesión anterior.
    pub fn tick(
        &mut self,
        window_title: String,
        process_name: String,
        application_name: String,
        now: u64,
    ) -> Option<Session> {
        let is_idle = self.state == ActivityState::Idle;

        let need_new = match &self.active {
            None => true,
            Some(s) => s.process_name != process_name,
        };

        if need_new {
            let closed = self.close_active(now);
            let session = Session::new(
                window_title,
                process_name,
                application_name,
                now,
                is_idle,
            );
            self.active = Some(session);
            closed
        } else if let Some(s) = self.active.as_mut() {
            // Mismo proceso: actualiza título e inactividad sin duplicar.
            s.window_title = window_title;
            s.is_idle = is_idle;
            None
        } else {
            None
        }
    }

    /// Cierra la sesión activa y la mueve al historial.
    pub fn close_active(&mut self, now: u64) -> Option<Session> {
        let mut session = self.active.take()?;
        session.finalize(now);
        let closed = Some(session.clone());
        self.sessions.push(session);
        closed
    }

    pub fn active_session(&self) -> Option<&Session> {
        self.active.as_ref()
    }

    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// Vacía el historial y la sesión activa (útil al detener el monitor).
    pub fn clear(&mut self) {
        self.active = None;
        self.sessions.clear();
    }

    /// Calcula estadísticas globales por aplicación.
    pub fn app_stats(&self) -> Vec<AppStats> {
        let mut map: HashMap<String, AppStats> = HashMap::new();

        // Agregar sesiones cerradas
        for s in &self.sessions {
            let entry = map.entry(s.process_name.clone()).or_insert(AppStats {
                application_name: s.application_name.clone(),
                process_name: s.process_name.clone(),
                total_duration_ms: 0,
                session_count: 0,
                first_seen: u64::MAX,
                last_seen: 0,
            });
            entry.total_duration_ms += s.duration_ms;
            entry.session_count += 1;
            entry.first_seen = entry.first_seen.min(s.started_at);
            entry.last_seen = entry.last_seen.max(s.ended_at);
        }

        // Incluir sesión activa si existe
        if let Some(s) = &self.active {
            let now = now_millis();
            let entry = map.entry(s.process_name.clone()).or_insert(AppStats {
                application_name: s.application_name.clone(),
                process_name: s.process_name.clone(),
                total_duration_ms: 0,
                session_count: 0,
                first_seen: u64::MAX,
                last_seen: 0,
            });
            entry.total_duration_ms += now.saturating_sub(s.started_at);
            entry.session_count += 1;
            entry.first_seen = entry.first_seen.min(s.started_at);
            entry.last_seen = entry.last_seen.max(now);
        }

        let mut stats: Vec<AppStats> = map.into_values().collect();
        stats.sort_by(|a, b| b.total_duration_ms.cmp(&a.total_duration_ms));
        stats
    }
}

/// Tiempo actual en milisegundos desde UNIX_EPOCH.
pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_new_session_on_first_tick() {
        let mut m = Monitor::new(60_000);
        // El primer tick no cierra ninguna sesión (no hay nada abierto): devuelve None.
        let closed = m.tick(
            "Notas".into(),
            "notepad.exe".into(),
            "Bloc de notas".into(),
            1_000,
        );
        assert!(closed.is_none());
        assert_eq!(m.active_session().unwrap().process_name, "notepad.exe");
    }

    #[test]
    fn does_not_duplicate_session_for_same_process() {
        let mut m = Monitor::new(60_000);
        m.tick("A".into(), "code.exe".into(), "VS Code".into(), 1_000);
        let started = m.tick("B (otra pestaña)".into(), "code.exe".into(), "VS Code".into(), 2_000);
        assert!(started.is_none());
        assert_eq!(m.active_session().unwrap().window_title, "B (otra pestaña)");
        assert_eq!(m.sessions().len(), 0);
    }

    #[test]
    fn closes_previous_session_when_process_changes() {
        let mut m = Monitor::new(60_000);
        m.tick("Editor".into(), "code.exe".into(), "VS Code".into(), 1_000);
        m.tick("Navegador".into(), "chrome.exe".into(), "Chrome".into(), 5_000);
        let sessions = m.sessions();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].process_name, "code.exe");
        assert_eq!(sessions[0].duration_ms, 4_000);
        assert_eq!(sessions[0].ended_at, 5_000);
        assert_eq!(m.active_session().unwrap().process_name, "chrome.exe");
    }

    #[test]
    fn locks_close_active_session() {
        let mut m = Monitor::new(60_000);
        m.tick("Editor".into(), "code.exe".into(), "VS Code".into(), 1_000);
        m.set_activity(ActivityState::Locked, 3_000);
        assert!(m.active_session().is_none());
        assert_eq!(m.sessions().len(), 1);
        assert_eq!(m.sessions()[0].duration_ms, 2_000);
    }

    #[test]
    fn idle_marks_session_as_idle() {
        let mut m = Monitor::new(60_000);
        m.set_activity(ActivityState::Idle, 1_000);
        m.tick("Repo".into(), "code.exe".into(), "VS Code".into(), 2_000);
        assert!(m.active_session().unwrap().is_idle);
    }

    #[test]
    fn clear_empties_everything() {
        let mut m = Monitor::new(60_000);
        m.tick("x".into(), "a.exe".into(), "A".into(), 1_000);
        m.tick("y".into(), "b.exe".into(), "B".into(), 2_000);
        m.clear();
        assert!(m.active_session().is_none());
        assert_eq!(m.sessions().len(), 0);
    }

    #[test]
    fn app_stats_aggregates_correctly() {
        let mut m = Monitor::new(60_000);
        m.tick("A".into(), "code.exe".into(), "VS Code".into(), 1_000);
        m.tick("B".into(), "chrome.exe".into(), "Chrome".into(), 5_000);
        m.tick("C".into(), "code.exe".into(), "VS Code".into(), 10_000);
        
        let stats = m.app_stats();
        assert_eq!(stats.len(), 2);
        
        // code.exe: 4000ms + sesión activa (desde 10000)
        let code_stats = stats.iter().find(|s| s.process_name == "code.exe").unwrap();
        assert_eq!(code_stats.total_duration_ms, 4_000 + now_millis() - 10_000);
        assert_eq!(code_stats.session_count, 2);
        
        // chrome.exe: 5000ms
        let chrome_stats = stats.iter().find(|s| s.process_name == "chrome.exe").unwrap();
        assert_eq!(chrome_stats.total_duration_ms, 5_000);
        assert_eq!(chrome_stats.session_count, 1);
    }
}
