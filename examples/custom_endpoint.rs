use wasmtime::*;
use wasm_obs_agent_lib::{instrument_module, TelemetryObserver};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Imprime para confirmar qué endpoint se está usando
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:4318/v1/traces".to_string());
    println!("🔗 Usando endpoint OTLP: {}", endpoint);

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

    let observer = TelemetryObserver::new(); // Lee OTEL_EXPORTER_OTLP_ENDPOINT automáticamente
    let funcs = instrument_module(&mut store, &module, observer)?;

    let add = funcs.get("add").expect("add exportada");
    let mut results = [Val::I32(0)];
    add.call(&mut store, &[Val::I32(10), Val::I32(20)], &mut results)?;
    println!("➕ add(10,20) = {}", results[0].i32().unwrap());

    let multiply = funcs.get("multiply").expect("multiply exportada");
    multiply.call(&mut store, &[Val::I32(6), Val::I32(7)], &mut results)?;
    println!("✖️ multiply(6,7) = {}", results[0].i32().unwrap());

    // Tiempo para que el batch exporter envíe los spans
    tokio::time::sleep(Duration::from_secs(3)).await;

    println!("✅ Ejemplo con endpoint custom completado.");

    Ok(())
}