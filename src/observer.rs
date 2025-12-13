use opentelemetry::global::ObjectSafeSpan;
use opentelemetry::trace::Tracer;
use opentelemetry::KeyValue;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::SystemTime;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;
use uuid::Uuid;
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
    fn record_event(&self, name: &str, attributes: Vec<KeyValue>);
}

pub struct TelemetryObserver {
    pub(crate) pending_starts: Mutex<HashMap<Uuid, u64>>,
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
            environment: std::env::var("OTEL_ENVIRONMENT").unwrap_or("development".to_string()),
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
        // Inicializamos el tracer global UNA SOLA VEZ
        crate::exporter::init_otlp_tracer(&self.endpoint, &self.service_name, &self.environment)
            .expect("Failed to initialize OTLP tracer");

        Arc::new(TelemetryObserver {
            pending_starts: Mutex::new(HashMap::new()),
        })
    }
}

impl TelemetryObserver {
    pub fn with_channel(sender: UnboundedSender<WasmSpan>) -> Self {
        Self {
            pending_starts: Mutex::new(HashMap::new()),
        }
    }
}

impl WasmObserver for TelemetryObserver {
    fn on_func_enter(&self, runtime_id: Uuid, _func_name: &str) {
        let start_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        self.pending_starts
            .lock()
            .unwrap()
            .insert(runtime_id, start_ns);
    }

    fn on_func_exit(&self, runtime_id: Uuid, func_name: &str, duration_ns: u64) {
        if let Some(start_ns) = self.pending_starts.lock().unwrap().remove(&runtime_id) {
            let end_ns = start_ns + duration_ns;

            let tracer = crate::exporter::get_tracer();

            let mut span = tracer.start(format!("wasm::{}", func_name));
            span.set_attribute(KeyValue::new("wasm.runtime_id", runtime_id.to_string()));
            span.end_with_timestamp(std::time::SystemTime::now() + Duration::from_nanos(end_ns));
        }
    }

    fn record_event(&self, name: &str, attributes: Vec<KeyValue>) {
        let tracer = crate::exporter::get_tracer();

        let mut span = tracer.start(format!("event::{}", name));

        for attr in attributes {
            span.set_attribute(attr);
        }

        span.end();
    }
}
