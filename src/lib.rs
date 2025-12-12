pub mod exporter;
pub mod instrument;
pub mod observer;
pub mod wrapper;
pub use instrument::instrument_module;
pub use observer::{TelemetryObserver, TelemetryObserverBuilder, WasmObserver};
pub use wrapper::ObservedInstance;

impl TelemetryObserver {
    pub fn new() -> std::sync::Arc<Self> {
        TelemetryObserverBuilder::new().build()
    }
}
