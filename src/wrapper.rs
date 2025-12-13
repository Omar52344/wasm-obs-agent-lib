// src/wrapper.rs
use crate::observer::WasmObserver;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use wasmtime::*;

pub struct ObservedInstance<T> {
    inner: Instance,
    store: *mut Store<T>,
    observer: Arc<dyn WasmObserver>, // Aquí está el cambio clave
    cache: RefCell<HashMap<String, Func>>,
}
unsafe impl<T> Send for ObservedInstance<T> where T: Send {}
impl<T> ObservedInstance<T>
where
    T: Send + 'static,
{
    pub fn new(
        store: &mut Store<T>,
        module: &Module,
        observer: Arc<impl WasmObserver + 'static>,
    ) -> Result<Self, anyhow::Error> {
        let inner = Instance::new(&mut *store, module, &[])?;

        Ok(Self {
            inner,
            store: store as *mut _,
            observer: observer as Arc<dyn WasmObserver>, // Conversión explícita
            cache: RefCell::new(HashMap::new()),
        })
    }

    pub async fn new_async(
        store: &mut Store<T>,
        linker: &Linker<T>,
        module: &Module,
        observer: Arc<dyn WasmObserver>,
    ) -> Result<Self, anyhow::Error> {
        let inner = linker.instantiate_async(&mut *store, module).await?;

        Ok(Self {
            inner,
            store: store as *mut _,
            observer,
            cache: RefCell::new(HashMap::new()),
        })
    }

    pub fn get_func(&self, store: &mut Store<T>, name: &str) -> Option<Func> {
        let store_mut: &mut Store<T> = unsafe { &mut *self.store };

        {
            let cache = self.cache.borrow();
            if let Some(func) = cache.get(name) {
                return Some(func.clone());
            }
        }

        let original = self.inner.get_func(&mut *store_mut, name)?;
        let name_owned = name.to_string();
        let observer = self.observer.clone(); // Ahora es Arc<dyn WasmObserver>

        let instrumented = instrument_function(store_mut, original, observer, name_owned);

        if let Ok(func) = &instrumented {
            self.cache
                .borrow_mut()
                .insert(name.to_string(), func.clone());
        }

        instrumented.ok()
    }

    pub fn inner(&self) -> &Instance {
        &self.inner
    }

    pub fn get_export(&self, store: &mut Store<T>, name: &str) -> Option<Extern> {
        self.inner.get_export(&mut *store, name)
    }
}

// Cambiamos la firma para aceptar Arc<dyn WasmObserver>
fn instrument_function<T>(
    store: &mut Store<T>,
    original: Func,
    observer: Arc<dyn WasmObserver>,
    name: String,
) -> Result<Func, anyhow::Error>
where
    T: Send + 'static,
{
    let ty = original.ty(&mut *store);

    Ok(Func::new_async(
        store,
        ty.clone(),
        move |mut caller, params, results| {
            let observer = observer.clone();
            let name = name.clone();
            let original = original.clone();

            Box::new(async move {
                let runtime_id = uuid::Uuid::new_v4();
                let start = std::time::Instant::now();

                observer.on_func_enter(runtime_id, &name);

                let result = original.call_async(&mut caller, params, results).await;

                let duration = start.elapsed().as_nanos() as u64;
                observer.on_func_exit(runtime_id, &name, duration);

                result.map_err(Into::into)
            })
        },
    ))
}
