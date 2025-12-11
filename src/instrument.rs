use crate::observer::WasmObserver;
use wasmtime::*;
use std::sync::Arc;
use std::collections::HashMap;
use anyhow::Result;

/// Retorna un HashMap con las funciones exportadas ya instrumentadas
pub fn instrument_module<T>(
    store: &mut Store<T>,
    module: &Module,
    observer: Arc<impl WasmObserver + 'static>,
) -> Result<HashMap<String, Func>>
where
    T: Send + 'static,
{
    // Reborrow explícito para evitar el move error
    let instance = Instance::new(&mut *store, module, &[])?;

    let mut instrumented_funcs = HashMap::new();

    for export in module.exports() {
        if let ExternType::Func(_) = export.ty() {
            // Reborrow seguro
            if let Some(original_func) = instance.get_func(&mut *store, export.name()) {
                let name = export.name().to_string();
                let instrumented = instrument_function(&mut *store, original_func, observer.clone(), name)?;
                instrumented_funcs.insert(export.name().to_string(), instrumented);
            }
        }
        // Non-func exports (memory, global, etc.) se acceden con instance.get_export(&mut *store, ...) si necesitas
    }

    Ok(instrumented_funcs)
}

fn instrument_function<T>(
    store: &mut Store<T>,
    original: Func,
    observer: Arc<impl WasmObserver + 'static>,
    name: String,
) -> Result<Func>
where
    T: Send + 'static,
{
    // Reborrow si necesario
    let ty = original.ty(&mut *store);

    Ok(Func::new(store, ty.clone(), move |mut caller: Caller<'_, T>, params: &[Val], results: &mut [Val]| -> Result<()> {
        let runtime_id = uuid::Uuid::new_v4();
        let start = std::time::Instant::now();

        observer.on_func_enter(runtime_id, &name);

        let result = original.call(&mut caller, params, results);

        let duration = start.elapsed().as_nanos() as u64;
        observer.on_func_exit(runtime_id, &name, duration);

        result.map_err(Into::into)
    }))
}