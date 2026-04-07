//! Future **Audio Engine** subprocess: dedicated real-time audio I/O, output device selection,
//! and insertable effects/plugins — isolated from the UI process for stability and lower latency.
//!
//! Planned integration (not implemented yet):
//! - Spawn or attach to a long-lived helper binary / sidecar
//! - IPC: transport control, device list, plugin chain graph, metering
//! - The WebView keeps preview/scan UX; engine owns final mix-out when routing is enabled
//!
//! This module is a placeholder so the crate graph reserves the name and docs live next to the app.

/// Placeholder until the engine process exists.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioEngineStub {
    pub state: String,
}

impl Default for AudioEngineStub {
    fn default() -> Self {
        Self {
            state: "not_started".to_string(),
        }
    }
}
