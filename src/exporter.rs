// src/exporter.rs
use opentelemetry::global;
use opentelemetry::KeyValue;
use opentelemetry_otlp::SpanExporter;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use std::time::Duration;

pub fn init_otlp_tracer(
    endpoint: &str,
    service_name: &str,
    environment: &str,
) -> Result<SdkTracerProvider, anyhow::Error> {
    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", service_name.to_string()),
            KeyValue::new("environment", environment.to_string()),
        ])
        .build();

    let exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .build()?;

    let tracer_provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter) // simple_exporter envía inmediatamente (perfecto para MVP)
        .with_resource(resource)
        .build();

    global::set_tracer_provider(tracer_provider.clone());

    println!("✅ OTLP tracer inicializado (endpoint: {})", endpoint);

    Ok(tracer_provider)
}
pub fn get_tracer() -> opentelemetry::global::BoxedTracer {
    global::tracer("orches")
}
