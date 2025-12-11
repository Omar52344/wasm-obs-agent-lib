//! wasm-obs-agent-lib
//! Auto-instrumentación automática de módulos WASM con OpenTelemetry
//!
//! Uso rápido:
//! ```no_run
//! use wasm_obs_agent_lib::{instrument_module, TelemetryObserver};
//! let instance = instrument_module(&mut store, &module, TelemetryObserver::new())?;
//! ```

pub mod observer;
pub mod exporter;
pub mod instrument;

pub use instrument::instrument_module;
pub use observer::{TelemetryObserver, WasmObserver, WasmSpan};

use std::env;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Extensión para TelemetryObserver: creación fácil con exporter automático
impl TelemetryObserver {
    /// Crea un observer y lanza automáticamente el exporter OTLP en background
    /// Usa la variable de entorno OTEL_EXPORTER_OTLP_ENDPOINT
    /// Default: http://127.0.0.1:4318/v1/traces
    pub fn new() -> Arc<Self> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = oneshot::channel();

        let endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:4318/v1/traces".to_string());

        let observer = Arc::new(Self::with_channel(sender.clone()));

        // Lanzamos el exporter en background
        tokio::spawn(crate::exporter::run_otlp_exporter(receiver, endpoint, ready_tx));

        // Opcional: esperamos a que esté listo (no bloqueamos indefinidamente)
        tokio::task::spawn_blocking(move || {
            let _ = ready_rx.blocking_recv();
        });
        observer
    }

    /// Para casos avanzados: crea el observer con un sender existente
    pub fn with_channel(sender: tokio::sync::mpsc::UnboundedSender<crate::observer::WasmSpan>) -> Self {
        Self {
            pending_starts: std::sync::Mutex::new(std::collections::HashMap::new()),
            sender,
        }
    }
}