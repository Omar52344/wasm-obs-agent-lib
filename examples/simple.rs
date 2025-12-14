use once_cell::sync::Lazy;
use opentelemetry::KeyValue;
use std::time::Duration;
use std::{path::Path, sync::Arc};
use wasm_obs_agent_lib::{
    ObservedInstance, TelemetryObserver, TelemetryObserverBuilder, WasmObserver,
};
use wasmtime::*;
use wasmtime_wasi::preview1::WasiP1Ctx;
use wasmtime_wasi::preview1::{self, wasi_snapshot_preview1};
use wasmtime_wasi::WasiCtxBuilder;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;

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

    let wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_env()
        .build_p1(); //
    let mut store = Store::new(&engine, wasi);


        let observer: Lazy<Arc<TelemetryObserver>> = Lazy::new(|| {
        TelemetryObserverBuilder::new()
            .with_service_name("orches")
            .with_environment("runtime")
            .with_endpoint("http://127.0.0.1:4318/v1/traces")
            .build()
    });
    //evento simple de error
    observer.record_event(
        "wasm_execution_failed",
        vec![
            KeyValue::new("error", "invalid_input"),
            KeyValue::new("module", "payment.wasm"),
        ],
    );
    //let funcs = instrument_module(&mut store, &module, observer)?;
    let mut linker = Linker::new(&engine);

    wasi_snapshot_preview1::add_to_linker(&mut linker, |ctx: &mut WasiP1Ctx| ctx)?;
    // Pruebas
    let instance =
        ObservedInstance::new_async(&mut store, &linker, &module, observer.clone()).await?;
    
    
    
    let func = instance
        .get_func(&mut store, "add")
        .ok_or_else(|| anyhow::anyhow!("Function '{}' not found", "add"))?;

    let func = func.typed::<(i32, i32), i32>(&store)?;
    let result = func.call_async(&mut store, (5, 3)).await?;


    println!("➕ add() = {}", result);



    tokio::time::sleep(Duration::from_secs(3)).await;

    println!("✅ Ejemplo completado. Revisa Jaeger para ver las traces.");

    Ok(())
}
