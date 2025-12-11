use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct WasmSpan {
    pub runtime_id: Uuid,
    pub function_name: String,
    pub start_time_ns: u64,
    pub end_time_ns: u64,
}

pub trait WasmObserver: Send + Sync + 'static {
    fn on_func_enter(&self, runtime_id: Uuid, func_name: &str);
    fn on_func_exit(&self, runtime_id: Uuid, func_name: &str, duration_ns: u64);
}

pub struct TelemetryObserver {
    pub(crate) pending_starts: Mutex<HashMap<Uuid, u64>>,
    pub(crate) sender: UnboundedSender<WasmSpan>,
}

impl WasmObserver for TelemetryObserver {
    fn on_func_enter(&self, runtime_id: Uuid, _func_name: &str) {
        let start_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        self.pending_starts.lock().unwrap().insert(runtime_id, start_ns);
    }

    fn on_func_exit(&self, runtime_id: Uuid, func_name: &str, duration_ns: u64) {
        if let Some(start_ns) = self.pending_starts.lock().unwrap().remove(&runtime_id) {
            let end_ns = start_ns + duration_ns;
            let span = WasmSpan {
                runtime_id,
                function_name: func_name.to_string(),
                start_time_ns: start_ns,
                end_time_ns: end_ns,
            };
            let _ = self.sender.send(span);
        }
    }
}