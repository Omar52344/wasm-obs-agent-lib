pub mod observer;
pub mod exporter;
pub mod instrument;

pub use instrument::instrument_module;
pub use observer::{TelemetryObserver, TelemetryObserverBuilder, WasmObserver};

impl TelemetryObserver {
    pub fn new() -> std::sync::Arc<Self> {
        TelemetryObserverBuilder::new().build()
    }
}