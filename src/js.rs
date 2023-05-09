use crate::db::DBManager;
use crate::web_dynamics::DynamicsManager;
use crate::{js_functions, nino_constants};
use deno_core::error::AnyError;
use deno_core::{
    anyhow::Error, futures::FutureExt, url::Url, Extension, InspectorSessionProxy, JsRuntime,
    ModuleLoader, ModuleSource, ModuleSourceFuture, ModuleSpecifier, ModuleType, OpDecl, OpState,
    RuntimeOptions,
};
use deno_core::{v8, FastString, ResolutionKind};
use deno_runtime::deno_broadcast_channel::InMemoryBroadcastChannel;
use deno_runtime::deno_web::BlobStore;
use deno_runtime::inspector_server::InspectorServer;
use deno_runtime::permissions::PermissionsContainer;
use deno_runtime::worker::{MainWorker, WorkerOptions};
use deno_runtime::BootstrapOptions;
use http_types::Response;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::{pin::Pin, rc::Rc, task::Context, task::Poll};
//use tokio::macros::support::poll_fn;

/// need to call start() to begin js threads
#[derive(Clone)]
pub struct JavaScriptManager {
    thread_count: u16,
    inspector_port: u16,
    db: DBManager,
    dynamics: Arc<DynamicsManager>,
}

static JS_INSTANCE: Mutex<Option<JavaScriptManager>> = Mutex::new(None);

impl JavaScriptManager {
    pub fn instance(
        thread_count: u16,
        inspector_port: u16,
        db: Option<DBManager>,
        dynamics: Option<Arc<DynamicsManager>>,
    ) -> JavaScriptManager {
        {
            let mut inst = JS_INSTANCE.lock().unwrap();
            if inst.is_none() {
                init_platform(thread_count);
                let this = JavaScriptManager {
                    thread_count,
                    inspector_port,
                    db: db.unwrap(),
                    dynamics: dynamics.unwrap().clone(),
                };
                inst.replace(this);
            }
        }
        JS_INSTANCE.lock().unwrap().as_mut().unwrap().clone()
    }

    pub async fn start() {
        let (thread_count, inspector_port) = {
            let instance = Self::instance(0, 0, None, None);
            (instance.thread_count, instance.inspector_port)
        };

        //for _ in 0..thread_count {
        //tokio::spawn(
        if let Err(e) = run_deno_main_thread(
            module_loader,
            js_functions::get_javascript_ops,
            Self::create_js_context_state,
            nino_constants::MAIN_MODULE,
            inspector_port,
        )
        .await
        {
            println!("ERROR: {}", e.to_string());
        }
        //);
        //}

        /*
        for _ in 0..thread_count {
            tokio::spawn(async {
                let main_module = Self::get_main_module().await;
                if let Err(e) = poll_fn(|cx| Self::start_deno_thread(cx, main_module.clone())).await
                {
                    println!("ERROR: {}", e.to_string());
                }
            });
        }
        */
    }

    fn create_js_context_state(state: &mut OpState) -> () {
        static JS_THREAD_ID: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);

        let js = JavaScriptManager::instance(0, 0, None, None);

        let mut bunding = JS_INSTANCE.lock().unwrap();
        let inst = bunding.as_mut().unwrap();
        state.put(js_functions::JSTask {
            id: JS_THREAD_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst) as u32,
            db: inst.db.clone(),
            web_task_rx: inst.dynamics.get_web_task_rx(),
            web_task: None,
            response: Response::new(200),
            dynamics: js.dynamics.clone(),
            module: String::from(""),
            closed: false,
        });
    }
    /*
    fn start_deno_thread(cx: &mut Context, main_module: String) -> Poll<Result<(), Error>> {
        run_deno_thread(
            cx,
            Rc::new(DBModuleLoader {}),
            js_functions::get_javascript_ops,
            Self::create_js_context_state,
            main_module.as_str(),
            None,
            None,
        )
    }
    */
}

/// this is used to create the v8 runtime :
/// it is intilaized only once and lives through the livetime of the applocation
pub fn init_platform(thread_count: u16) {
    static INIT_PLATFORM: std::sync::Mutex<bool> = std::sync::Mutex::new(false);
    {
        //init platform once
        let mut g_platform = INIT_PLATFORM.lock().unwrap();
        if !*g_platform {
            //init default platform
            let platform =
                Some(deno_core::v8::new_default_platform(thread_count as u32, false).make_shared());

            let loader = Rc::new(FNModuleLoader::new(module_loader));
            let ext = Extension::builder(nino_constants::PROGRAM_NAME)
                .ops(js_functions::get_javascript_ops())
                .build();
            let _r = JsRuntime::new(RuntimeOptions {
                v8_platform: platform,
                module_loader: Some(loader),
                extensions: vec![ext],
                will_snapshot: false,
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
        let instance = JS_INSTANCE.lock().unwrap().as_mut().unwrap().clone();
        match instance.dynamics.get_module_js(name.clone().as_str()).await {
            Some(code) => Ok(code),
            None => {
                let err_msg = format!(
                    "cannot load the main module '{}' from dynamics",
                    name.clone()
                );
                Err(Error::msg(err_msg))
            }
        }
    }
    .boxed_local()
}

/// used for loading js modules
pub struct FNModuleLoader {}

static FNMODULE_LOADER_FUNCTION: Mutex<Option<ModuleLoadingFunction>> = Mutex::new(None);

impl FNModuleLoader {
    fn new(module_loader: ModuleLoadingFunction) -> FNModuleLoader {
        {
            let mut fn_holder = FNMODULE_LOADER_FUNCTION.lock().unwrap();
            *fn_holder = Some(module_loader);
        }
        FNModuleLoader {}
    }

    async fn async_load(module_name: String) -> Result<ModuleSource, Error> {
        let code = {
            let fn_holder = FNMODULE_LOADER_FUNCTION.lock().unwrap();
            if let Some(func) = *fn_holder {
                func(module_name.clone()).boxed_local().await?
            } else {
                return Err(Error::msg("No loading function in FNModuleLoaderFunction"));
            }
        };

        let module_type = ModuleType::JavaScript;
        // ModuleType::Json
        let code = FastString::from(String::from(code)); //code.as_bytes().to_vec().into_boxed_slice();
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
        let url;
        if specifier.starts_with(nino_constants::MODULE_URI) {
            url = Url::parse(&specifier)?;
        } else {
            let url_str = format!("{}{}", nino_constants::MODULE_URI, specifier);
            url = Url::parse(&url_str)?;
        }
        Ok(url)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: std::option::Option<&deno_core::url::Url>,
        _is_dyn_import: bool,
    ) -> Pin<Box<ModuleSourceFuture>> {
        //let module_specifier = module_specifier.clone();
        let module_path = &module_specifier.path()[1..];
        println!("load module: {}", &module_path);
        Self::async_load(String::from(module_path)).boxed_local()
    }
}

pub struct Task {
    pub id: u32,
}

fn create_state(state: &mut OpState) -> () {
    state.put(Task { id: 0 });
    ()
}

pub enum RetrievedV8Value<'s> {
    Value(v8::Local<'s, v8::Value>),
    Error(v8::Local<'s, v8::Value>),
    Promise(v8::Local<'s, v8::Promise>),
}

// This is done as a macro so that Rust can reuse the borrow on the scope,
// instead of treating the returned value's reference to the scope as a new mutable borrow.
/*
macro_rules! extract_promise {
    ($scope: expr, $v: expr) => {
        // If it's a promise, try to get the value out.
        if $v.is_promise() {
            let promise = v8::Local::<v8::Promise>::try_from($v).unwrap();
            match promise.state() {
                v8::PromiseState::Pending => RetrievedV8Value::Promise(promise),
                v8::PromiseState::Fulfilled => RetrievedV8Value::Value(promise.result(&mut $scope)),
                v8::PromiseState::Rejected => RetrievedV8Value::Error(promise.result(&mut $scope)),
            }
        } else {
            RetrievedV8Value::Value($v)
        }
    };
}
*/

// old one using the deno runtime
pub fn run_deno_thread(
    cx: &mut Context,
    module_loader: Rc<dyn ModuleLoader>,
    get_ops: fn() -> Vec<OpDecl>,
    create_state: fn(state: &mut OpState) -> (),
    javascript_source_code: &str,
    inspector_session_sx: Option<InspectorSessionProxy>,
) -> Poll<Result<(), Error>> {
    let need_inspector = inspector_session_sx.is_some();

    let mut runtime = JsRuntime::new(RuntimeOptions {
        module_loader: Some(module_loader),
        extensions: vec![Extension::builder(nino_constants::PROGRAM_NAME)
            .ops(get_ops())
            .state(create_state)
            .build()],
        will_snapshot: false,
        inspector: need_inspector,
        ..Default::default()
    });

    let code = FastString::from(String::from(javascript_source_code));
    let _result = runtime.execute_script("main", code)?;
    let r = runtime.poll_event_loop(cx, need_inspector);
    eprintln!("js thread done");
    r
}

fn get_error_class_name(e: &AnyError) -> &'static str {
    deno_runtime::errors::get_error_class_name(e).unwrap_or("Error")
}

// new one using deno MainWorker
pub async fn run_deno_main_thread(
    module_loader: ModuleLoadingFunction,
    get_ops: fn() -> Vec<OpDecl>,
    create_state: fn(state: &mut OpState) -> (),
    main_module: &str,
    inspector_port: u16,
) -> Result<(), Error> {
    let create_web_worker_cb = Arc::new(|_| {
        todo!("Web workers are not supported in the example");
    });
    let web_worker_event_cb = Arc::new(|_| {
        todo!("Web workers are not supported in the example");
    });

    let extensions = {
        let ext = Extension::builder("nino_extentions")
            .ops(get_ops())
            .state(create_state)
            .force_op_registration()
            .build();
        vec![ext]
    };

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

    let module_loader = Rc::new(FNModuleLoader::new(module_loader));

    let options = WorkerOptions {
        bootstrap: BootstrapOptions::default(),
        extensions,
        startup_snapshot: None,
        unsafely_ignore_certificate_errors: None,
        root_cert_store: None,
        seed: None,
        source_map_getter: None,
        format_js_error_fn: None,
        web_worker_preload_module_cb: web_worker_event_cb.clone(),
        web_worker_pre_execute_module_cb: web_worker_event_cb,
        create_web_worker_cb,
        module_loader,
        npm_resolver: None,
        get_error_class_fn: Some(&get_error_class_name),
        cache_storage_dir: None,
        origin_storage_dir: None,
        blob_store: BlobStore::default(),
        broadcast_channel: InMemoryBroadcastChannel::default(),
        shared_array_buffer_store: None,
        compiled_wasm_module_store: None,
        maybe_inspector_server,
        should_break_on_first_statement: false,
        should_wait_for_inspector_session: false,
        stdio: Default::default(),
    };

    let main_uri = format!("{}{}", nino_constants::MODULE_URI, main_module).to_owned();
    let main_module = Url::parse(main_uri.as_str())?;
    let permissions = PermissionsContainer::new(deno_runtime::permissions::Permissions::default());

    let mut worker = MainWorker::bootstrap_from_options(main_module.clone(), permissions, options);

    worker.execute_main_module(&main_module).await?;
    worker.run_event_loop(false).await?;

    Ok(())
}
