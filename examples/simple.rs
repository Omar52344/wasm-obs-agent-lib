use wasmtime::*;
use wasm_obs_agent_lib::{instrument_module, TelemetryObserver};
use std::time::Duration;

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

    let observer = TelemetryObserver::new(); // Aquí se lanza el exporter en Tokio
    let funcs = instrument_module(&mut store, &module, observer)?;

    // Pruebas
    let add = funcs.get("add").expect("add exportada");
    let mut results = [Val::I32(0)];
    add.call(&mut store, &[Val::I32(5), Val::I32(3)], &mut results)?;
    println!("➕ add(5,3) = {}", results[0].i32().unwrap());

    let multiply = funcs.get("multiply").expect("multiply exportada");
    multiply.call(&mut store, &[Val::I32(4), Val::I32(7)], &mut results)?;
    println!("✖️ multiply(4,7) = {}", results[0].i32().unwrap());

    // Dale tiempo al batch exporter para flush (500ms config)
    tokio::time::sleep(Duration::from_secs(3)).await;

    println!("✅ Ejemplo completado. Revisa Jaeger para ver las traces.");

    Ok(())
}