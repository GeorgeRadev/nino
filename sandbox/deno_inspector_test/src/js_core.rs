use deno_core::error::ModuleLoaderError;
use deno_core::{
    anyhow::Error, futures::FutureExt, url::Url, Extension, FastString, JsRuntime,
    ModuleLoadResponse, ModuleLoader, ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType,
    RequestedModuleType, ResolutionKind,
};
use deno_core::{PollEventLoopOptions, RuntimeOptions};
use deno_error::JsErrorBox;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

pub const MAIN_DB: &str = "_main";
pub const MODULE_URI: &str = "http://nino.db/";

#[derive(Clone)]
pub enum ExecuteMode {
    JsModule(Url),
    JsCode(String),
}

pub type ModuleLoadingFunction =
    fn(String) -> Pin<Box<dyn Future<Output = Result<String, Error>> + 'static>>;

pub type ExtentionsSupplier = fn() -> Vec<Extension>;

pub fn js_init(module_loader: ModuleLoadingFunction, thread_pool_size: u32) {
    FNMODULE_LOADER_FUNCTION.get_or_init(|| module_loader);
    let v8_platform =
        Some(deno_core::v8::new_default_platform(thread_pool_size, false).make_shared());
    // Initialize a runtime instance
    let mut _js_runtime = JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(FNModuleLoader {})),
        v8_platform,
        inspector: false,
        is_main: true,
        ..Default::default()
    });
}

pub struct FNModuleLoader {}

static FNMODULE_LOADER_FUNCTION: OnceLock<ModuleLoadingFunction> = OnceLock::new();

impl FNModuleLoader {
    async fn async_load(module_name: String) -> Result<ModuleSource, ModuleLoaderError> {
        let fn_holder = FNMODULE_LOADER_FUNCTION.get();
        match fn_holder {
            Some(module_loader) => match module_loader(module_name.clone()).await {
                Ok(code) => {
                    let module_type = ModuleType::JavaScript;
                    let code = ModuleSourceCode::String(FastString::from(code));
                    let module_string =
                        Url::parse(&format!("{}{}", MODULE_URI, module_name)).unwrap();
                    let module = ModuleSource::new(module_type, code, &module_string, None);
                    Ok(module)
                }
                Err(_) => Err(ModuleLoaderError::NotFound),
            },
            None => Err(ModuleLoaderError::NotFound),
        }
    }
}

impl ModuleLoader for FNModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        _referrer: &str,
        _kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, ModuleLoaderError> {
        let url_str = if specifier.starts_with(MODULE_URI) {
            specifier
        } else {
            &format!("{}{}", MODULE_URI, specifier)
        };
        match Url::parse(url_str) {
            Ok(url) => Ok(url),
            Err(_error) => Err(JsErrorBox::generic(format!("cannot parse: {}", url_str)).into()),
        }
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
        _requested_module_type: RequestedModuleType,
    ) -> ModuleLoadResponse {
        let module_path = &module_specifier.path()[1..];
        ModuleLoadResponse::Async(Self::async_load(String::from(module_path)).boxed_local())
    }
}

pub fn start_js_thread(
    extensions_supplier: ExtentionsSupplier,
    execute: ExecuteMode,
    forever: bool,
    inspector_port: u16,
) -> Result<std::thread::JoinHandle<()>, Error> {
    static JS_THREAD_ID: std::sync::atomic::AtomicI16 = std::sync::atomic::AtomicI16::new(1);
    let id: i16 = JS_THREAD_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    let thread_name = format!("js-thread-{}", id).to_string();
    let thread = std::thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .max_blocking_threads(32)
                .build()
                .unwrap();
            let local = tokio::task::LocalSet::new();
            local.block_on(
                &rt,
                start_js(extensions_supplier, execute, forever, inspector_port),
            )
        })?;
    Ok(thread)
}

async fn start_js(
    extensions_supplier: ExtentionsSupplier,
    execute: ExecuteMode,
    forever: bool,
    inspector_port: u16,
) {
    loop {
        if let Err(error) = _start_js(extensions_supplier, execute.clone(), inspector_port).await {
            println!("ERROR: {}", error)
        }
        if !forever {
            break;
        }
    }
}

async fn _start_js(
    extensions_supplier: ExtentionsSupplier,
    execute: ExecuteMode,
    inspector_port: u16,
) -> Result<(), Error> {
    // inspector
    let inspector = inspector_port > 0;
    let inspector_server = if inspector {
        Some(Arc::new(js_inspector::InspectorServer::new(
            inspector_port,
        )?))
    } else {
        None
    };

    // Initialize a runtime instance
    let mut js_runtime = JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(FNModuleLoader {})),
        extensions: extensions_supplier(),
        inspector: inspector_server.is_some(),
        is_main: true,
        ..Default::default()
    });

    if let Some(inspector_server) = inspector_server.clone() {
        inspector_server.register_inspector("debugger".into(), &mut js_runtime, false);
        let op_state = js_runtime.op_state();
        let inspector = js_runtime.inspector();
        op_state.borrow_mut().put(inspector);
    }

    match execute {
        ExecuteMode::JsModule(main_module) => {
            // start the mail module and event loop
            let mod_id = js_runtime.load_main_es_module(&main_module).await?;
            js_runtime.mod_evaluate(mod_id).await?;
            //js_runtime.run_event_loop(Default::default()).await?;
            js_runtime
                .run_event_loop(PollEventLoopOptions {
                    wait_for_inspector: false,
                    ..Default::default()
                })
                .await?;
        }
        ExecuteMode::JsCode(code) => {
            let source_code = FastString::from(code.to_owned());
            js_runtime.execute_script("js_code", source_code)?;
            js_runtime
                .run_event_loop(PollEventLoopOptions {
                    wait_for_inspector: false,
                    ..Default::default()
                })
                .await?;
        }
    }

    drop(inspector_server);

    Ok(())
}
