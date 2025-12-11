use opentelemetry::{
    global,
    trace::{SpanBuilder, SpanKind,Tracer},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime::Tokio,
    trace::{BatchConfigBuilder, config as trace_config},
    Resource,
};
use tokio::sync::mpsc;
use crate::observer::WasmSpan;
use std::time::{Duration, UNIX_EPOCH};
use tokio::sync::oneshot;

pub async fn run_otlp_exporter(
    mut rx: mpsc::UnboundedReceiver<WasmSpan>,
    endpoint: String,
    service_name: String,
    environment: String,
) {
    println!("🛠️ Exporter task iniciada");
    println!("🛠️ Configurando exportador OTLP a: {}", endpoint);

    let resource = Resource::new(vec![
        KeyValue::new("service.name", service_name),
        KeyValue::new("environment", environment),
    ]);

    let exporter_builder = opentelemetry_otlp::new_exporter()
        .http()
        .with_endpoint(&endpoint);

    let pipeline_result = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter_builder)
        .with_trace_config(trace_config().with_resource(resource))
        .with_batch_config(
            BatchConfigBuilder::default()
                .with_scheduled_delay(Duration::from_millis(500))
                .with_max_export_batch_size(512)
                .with_max_queue_size(2048)
                .build()
        )
        .install_batch(Tokio);

    let tracer = match pipeline_result {
        Ok(_) => {
            println!("✅ Pipeline OTLP instalado correctamente");
            global::tracer("wasm-obs-agent")
        }
        Err(e) => {
            eprintln!("❌ Error instalando pipeline OTLP: {}", e);
            return; // Salir temprano si falla
        }
    };

    println!("🛠️ Exportador OTLP listo y esperando spans...");

    while let Some(span) = rx.recv().await {
        println!("📡 Procesando span para función: {}", span.function_name);

        if span.end_time_ns <= span.start_time_ns {
            println!("⚠️ Ignorando span con duración inválida (<=0) para {}", span.function_name);
            continue;
        }

        let start_time = UNIX_EPOCH + Duration::from_nanos(span.start_time_ns);
        let end_time = UNIX_EPOCH + Duration::from_nanos(span.end_time_ns);

        let mut builder = SpanBuilder::from_name(format!("wasm::{}", span.function_name));
        builder.span_kind = Some(SpanKind::Internal);
        builder.start_time = Some(start_time);
        builder.end_time = Some(end_time);
        
        builder.attributes = Some(vec![
            KeyValue::new("wasm.runtime_id", span.runtime_id.to_string()),
        ]);

        println!(
            "📤 Enviando span completo: {} (duración: {} ns)",
            span.function_name,
            span.end_time_ns - span.start_time_ns
        );

        tracer.build(builder); // Span se finaliza automáticamente
    }

    global::shutdown_tracer_provider();
    println!("✅ Shutdown de OTLP completado.");
}