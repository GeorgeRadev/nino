use crate::db_notification::Notifier;
use crate::db_settings::SettingsManager;
use crate::db_transactions::{TransactionManager, TransactionSession};
use crate::js_worker::{MainWorker, WorkerOptions};
use crate::nino_constants::info;
use crate::web_responses::ResponseManager;
use crate::{js_functions, nino_constants};
use deno_runtime::deno_core::{
    anyhow::Error, futures::FutureExt, url::Url, Extension, FastString, JsRuntime,
    ModuleLoadResponse, ModuleLoader, ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType,
    OpDecl, OpState, RequestedModuleType, ResolutionKind, RuntimeOptions,
};
use deno_runtime::{
    deno_broadcast_channel::InMemoryBroadcastChannel, inspector_server::InspectorServer,
    BootstrapOptions,
};
use std::future::Future;
use std::net::SocketAddr;
use std::sync::{Arc, OnceLock};
use std::thread::{self};
use std::{pin::Pin, rc::Rc};
//use tokio::macros::support::poll_fn;

/// need to call start() to begin js threads
pub struct JavaScriptManager {
    thread_count: usize,
    inspector_port: u16,
    connection_string: String,
    dynamics: Arc<ResponseManager>,
    settings: Arc<SettingsManager>,
    notifier: Arc<Notifier>,
}

static JS_INSTANCE: OnceLock<JavaScriptManager> = OnceLock::new();

impl JavaScriptManager {
    /**
     * Create and initialize Singleton Manager instance.
     */
    pub fn create(
        thread_count: usize,
        inspector_port: u16,
        connection_string: String,
        dynamics: Arc<ResponseManager>,
        settings: Arc<SettingsManager>,
    ) {
        JS_INSTANCE.get_or_init(|| {
            init_platform(thread_count, module_loader);

            let dynamics = dynamics;

            let mut this = JavaScriptManager {
                thread_count,
                inspector_port,
                connection_string,
                notifier: dynamics.get_notifier(),
                dynamics,
                settings,
            };
            // start all threads
            this.start();

            this
        });
    }

    // start all js processing threads
    // inspector port is attached only to the first js instance
    // for developing purposes use single js instance and debugger will attach to it
    fn start(&mut self) {
        let thread_count = self.thread_count;
        let inspector_port = self.inspector_port;

        for i in 0..thread_count {
            let builder = thread::Builder::new().name(format!("JS Thread {}", i).to_string());
            match builder.spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                if let Err(error) = rt.block_on(run_deno_main_thread(
                    module_loader,
                    js_functions::get_nino_functions(),
                    Self::create_js_context_state,
                    nino_constants::MAIN_MODULE,
                    None,
                    if inspector_port > 0 {
                        inspector_port + i as u16
                    } else {
                        0
                    },
                    true,
                )) {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
            }) {
                Ok(_jh) => {
                    //self.join_handlers.push(jh);
                }
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
            }
        }
    }

    /**
     * Creates the js thread context with sequential id, and attach all managers inside.
     * Allocating resources to the thread and releasing them must be handled in the main javascript try finaly block.
     */
    fn create_js_context_state(state: &mut OpState) {
        static JS_THREAD_ID: std::sync::atomic::AtomicI16 = std::sync::atomic::AtomicI16::new(-1);
        let id: i16 = JS_THREAD_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let js = JS_INSTANCE.get().unwrap();

        state.put(js_functions::JSContext {
            id,
            web_task_rx: js.dynamics.get_web_task_rx(),
            dynamics: js.dynamics.clone(),
            notifier: js.notifier.clone(),
            broadcast_messages: Vec::with_capacity(8),
            task: None,
        });
        let session = TransactionManager::get_transaction_session(js.connection_string.clone());
        state.put::<TransactionSession>(session);
        state.put::<SettingsManager>(js.settings.as_ref().clone())
    }

    pub async fn run(code: &str) -> Result<(), Error> {
        run_deno_main_thread(
            module_loader,
            js_functions::get_nino_functions(),
            Self::create_js_context_state,
            nino_constants::MAIN_MODULE,
            Some(code),
            0,
            false,
        )
        .await
    }
}
/// this is used to create the v8 runtime :
/// it is intilaized only once and lives through the livetime of the applocation
pub fn init_platform(thread_count: usize, module_loader: ModuleLoadingFunction) {
    static INIT_PLATFORM: std::sync::Mutex<bool> = std::sync::Mutex::new(false);
    {
        //init platform once
        let mut g_platform = INIT_PLATFORM.lock().unwrap();
        if !*g_platform {
            //init default platform
            let platform = Some(
                deno_runtime::deno_core::v8::new_default_platform(thread_count as u32, false)
                    .make_shared(),
            );

            let loader = Rc::new(FNModuleLoader::new(module_loader));
            let extensions = create_extensions(js_functions::get_nino_functions(), |_state| {});

            let _r = JsRuntime::new(RuntimeOptions {
                v8_platform: platform,
                module_loader: Some(loader),
                extensions,
                inspector: false,
                ..Default::default()
            });

            *g_platform = true;
        }
    }
}

// structure for storing async function for loading module code
type ModuleLoadingFunction =
    fn(String) -> Pin<Box<dyn Future<Output = Result<String, Error>> + 'static>>;

fn module_loader(name: String) -> Pin<Box<dyn Future<Output = Result<String, Error>> + 'static>> {
    async move {
        let instance = JS_INSTANCE.get().unwrap();
        let content = instance.dynamics.get_response_javascript(name.clone().as_str()).await?;
        let content_str = String::from_utf8(content)?;
        Ok(content_str)
    }
    .boxed_local()
}

/// used for loading js modules
pub struct FNModuleLoader {}

static FNMODULE_LOADER_FUNCTION: OnceLock<ModuleLoadingFunction> = OnceLock::new();

impl FNModuleLoader {
    pub fn new(module_loader: ModuleLoadingFunction) -> FNModuleLoader {
        FNMODULE_LOADER_FUNCTION.get_or_init(|| module_loader);
        FNModuleLoader {}
    }

    async fn async_load(module_name: String) -> Result<ModuleSource, Error> {
        let code = {
            let fn_holder = FNMODULE_LOADER_FUNCTION.get();
            if let Some(func) = fn_holder {
                func(module_name.clone()).boxed_local().await?
            } else {
                return Err(Error::msg("No loading function in FNModuleLoaderFunction"));
            }
        };

        let module_type = ModuleType::JavaScript;
        let code = ModuleSourceCode::String(FastString::from(code));
        let module_string =
            Url::parse(format!("{}{}", nino_constants::MODULE_URI, module_name).as_str())?;
        let module = ModuleSource::new(module_type, code, &module_string);
        Ok(module)
    }
}

impl ModuleLoader for FNModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        _referrer: &str,
        _kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, Error> {
        let url = if specifier.starts_with(nino_constants::MODULE_URI) {
            Url::parse(specifier)?
        } else {
            let url_str = format!("{}{}", nino_constants::MODULE_URI, specifier);
            Url::parse(&url_str)?
        };
        Ok(url)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
        _requested_module_type: RequestedModuleType,
    ) -> ModuleLoadResponse {
        let module_path = &module_specifier.path()[1..];
        info!("load module: {}", &module_path);
        ModuleLoadResponse::Async(Self::async_load(String::from(module_path)).boxed_local())
    }
}

fn get_error_class_name(e: &Error) -> &'static str {
    deno_runtime::errors::get_error_class_name(e).unwrap_or("Error")
}

pub fn create_extensions(
    ops: Vec<OpDecl>,
    state_fn: fn(state: &mut OpState) -> (),
) -> Vec<Extension> {
    let extention = Extension {
        name: "nino",
        ops: ops.into(),
        op_state_fn: Some(Box::new(state_fn)),
        ..Default::default()
    };
    vec![extention]
}

// new one using deno MainWorker
pub async fn run_deno_main_thread(
    module_loader: ModuleLoadingFunction,
    ops: Vec<OpDecl>,
    state_fn: fn(state: &mut OpState) -> (),
    main_module: &str,
    just_code: Option<&str>,
    inspector_port: u16,
    in_loop: bool,
) -> Result<(), Error> {
    let main_uri = format!("{}{}", nino_constants::MODULE_URI, main_module).to_owned();
    let main_module = Url::parse(main_uri.as_str())?;

    let maybe_inspector_server: Option<Arc<InspectorServer>> = {
        if inspector_port != 0 {
            let inspector_str = format!("127.0.0.1:{}", inspector_port);
            let inspector_address = inspector_str.parse::<SocketAddr>().unwrap();
            Some(Arc::new(InspectorServer::new(
                inspector_address,
                nino_constants::PROGRAM_NAME,
            )))
        } else {
            None
        }
    };

    loop {
        let extensions = create_extensions(ops.clone(), state_fn);

        let options = WorkerOptions {
            bootstrap: BootstrapOptions::default(),
            extensions,
            module_loader: Rc::new(FNModuleLoader::new(module_loader)),
            get_error_class_fn: Some(&get_error_class_name),
            broadcast_channel: InMemoryBroadcastChannel::default(),
            maybe_inspector_server: maybe_inspector_server.clone(),
            seed: None,
            startup_snapshot: None,
            source_map_getter: None,
            format_js_error_fn: None,
            shared_array_buffer_store: None,
            compiled_wasm_module_store: None,
            unsafely_ignore_certificate_errors: None,
            should_break_on_first_statement: false,
            should_wait_for_inspector_session: false,
            ..Default::default()
        };

        let mut worker = MainWorker::from_options(main_module.clone(), options);

        if just_code.is_none() {
            worker.execute_main_module(&main_module).await?;
            worker.run_event_loop(false).await?;
        } else {
            let source_code = FastString::from(just_code.unwrap().to_owned());
            worker.execute_script("js_code", source_code)?;
            worker.run_event_loop(false).await?;
        }

        if !in_loop {
            break;
        }
    }
    // maybe_inspector_server.unwrap();
    Ok(())
}
