use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::Arc;
use tokio::sync::oneshot;
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

pub struct TelemetryObserverBuilder {
    endpoint: String,
    service_name: String,
    environment: String,
}

impl TelemetryObserverBuilder {
    pub fn new() -> Self {
        Self {
            endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or("http://127.0.0.1:4318/v1/traces".to_string()),
            service_name: std::env::var("OTEL_SERVICE_NAME")
                .unwrap_or("wasm-obs-agent".to_string()),
            environment: std::env::var("OTEL_ENVIRONMENT")
                .unwrap_or("development".to_string()),
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }

    pub fn with_environment(mut self, env: impl Into<String>) -> Self {
        self.environment = env.into();
        self
    }

    pub fn build(self) -> Arc<TelemetryObserver> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        
        let observer = Arc::new(TelemetryObserver {
            pending_starts: Mutex::new(HashMap::new()),
            sender: sender.clone(),
        });

        tokio::spawn(crate::exporter::run_otlp_exporter(
            receiver,
            self.endpoint,
            self.service_name,
            self.environment,
        ));

        observer
    }
}

impl TelemetryObserver {
    pub fn with_channel(sender: UnboundedSender<WasmSpan>) -> Self {
        Self {
            pending_starts: Mutex::new(HashMap::new()),
            sender,
        }
    }
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