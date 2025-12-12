use opentelemetry::KeyValue;
use std::time::Duration;
use wasm_obs_agent_lib::{ObservedInstance, TelemetryObserverBuilder, WasmObserver};
use wasmtime::*;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let engine = Engine::default();
    let wat = r#"
        (module
            (func $add (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.add
            )
            (export "add" (func $add))
            
            (func $multiply (param i32 i32) (result i32)
                local.get 0
                local.get 1
                i32.mul
            )
            (export "multiply" (func $multiply))
        )
    "#;
    let module = Module::new(&engine, wat)?;
    let mut store = Store::new(&engine, ());

    /*let builder = TelemetryObserverBuilder::new()
        .with_service_name("payment-host")  // Nombre único por host
        .with_environment("production"); // Aquí se lanza el exporter en Tokio
        let observer = builder.build();
    */
    let observer = TelemetryObserverBuilder::new()
        .with_service_name("payment-host")
        .with_environment("production")
        .build();

    observer.record_event(
        "wasm_execution_success",
        vec![
            KeyValue::new("module", "payment.wasm"),
            KeyValue::new("status", "success"),
            KeyValue::new("duration_ms", "12"),
        ],
    );

    // O para error
    observer.record_event(
        "wasm_execution_failed",
        vec![
            KeyValue::new("error", "invalid_input"),
            KeyValue::new("module", "payment.wasm"),
        ],
    );

    //let funcs = instrument_module(&mut store, &module, observer)?;

    // Pruebas
    let intance = ObservedInstance::new(&mut store, &module, observer)?;
    let mut results = [Val::I32(0)];
    intance.get_func(&mut store, "add").unwrap().call(
        &mut store,
        &[Val::I32(5), Val::I32(3)],
        &mut results,
    )?;
    println!("➕ add(5,3) = {}", results[0].i32().unwrap());

    intance.get_func(&mut store, "multiply").unwrap().call(
        &mut store,
        &[Val::I32(4), Val::I32(7)],
        &mut results,
    )?;
    println!("✖️ multiply(4,7) = {}", results[0].i32().unwrap());
    // instance.observer.record_event(...) si lo expones, o usa el observer original
    // Dale tiempo al batch exporter para flush (500ms config)
    tokio::time::sleep(Duration::from_secs(3)).await;

    println!("✅ Ejemplo completado. Revisa Jaeger para ver las traces.");

    Ok(())
}
