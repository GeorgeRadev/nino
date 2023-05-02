use crate::db::DBManager;
use crate::web_dynamics::DynamicsManager;
use crate::{js_functions, nino_constants};
use deno_core::{
    anyhow::Error, futures::channel::oneshot::Sender, futures::FutureExt, url::Url, Extension,
    InspectorSessionProxy, JsRuntime, ModuleLoader, ModuleSource, ModuleSourceFuture,
    ModuleSpecifier, ModuleType, OpDecl, OpState, RuntimeOptions,
};
use deno_core::{v8, ResolutionKind};
use http_types::Response;
use std::sync::Arc;
use std::{pin::Pin, rc::Rc, task::Context, task::Poll};
use tokio::macros::support::poll_fn;

/// need to call start() to begin js threads
pub struct JavaScriptManager {
    thread_count: u16,
    db: DBManager,
    dynamics: Arc<DynamicsManager>,
}

impl Clone for JavaScriptManager {
    fn clone(&self) -> Self {
        Self {
            thread_count: self.thread_count,
            db: self.db.clone(),
            dynamics: self.dynamics.clone(),
        }
    }
}

static JS_INSTANCE: std::sync::Mutex<Option<JavaScriptManager>> = std::sync::Mutex::new(None);

impl JavaScriptManager {
    pub fn instance(
        thread_count: u16,
        db: Option<DBManager>,
        dynamics: Option<Arc<DynamicsManager>>,
    ) -> JavaScriptManager {
        {
            let mut inst = JS_INSTANCE.lock().unwrap();
            if inst.is_none() {
                init_platform(thread_count);
                let this = JavaScriptManager {
                    thread_count,
                    db: db.unwrap(),
                    dynamics: dynamics.unwrap().clone(),
                };
                inst.replace(this);
            }
        }
        JS_INSTANCE.lock().unwrap().as_mut().unwrap().clone()
    }

    pub async fn start() {
        let thread_count = {
            let instance = Self::instance(0, None, None);
            instance.thread_count
        };

        for _ in 0..thread_count {
            tokio::spawn(async {
                let main_module = Self::get_main_module().await;
                if let Err(e) = poll_fn(|cx| Self::start_deno_thread(cx, main_module.clone())).await
                {
                    println!("ERROR: {}", e.to_string());
                }
            });
        }
    }

    async fn get_main_module() -> String {
        let instance = Self::instance(0, None, None);
        match instance
            .dynamics
            .get_module_js(crate::nino_constants::MAIN_MODULE)
            .await
        {
            Some(code) => return code,
            None => panic!(
                "cannot load the main module '{}' from dynamics",
                crate::nino_constants::MAIN_MODULE
            ),
        }
    }

    fn create_js_context_state(state: &mut OpState) -> Result<(), deno_core::anyhow::Error> {
        static JS_THREAD_ID: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);

        let js = JavaScriptManager::instance(0, None, None);

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
        Ok(())
    }

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

            let loader = Rc::new(DBModuleLoader {});
            let ext = Extension::builder(nino_constants::PROGRAM_NAME)
                .ops(js_functions::get_javascript_ops())
                .state(create_state)
                .build();
            let _r = JsRuntime::new(RuntimeOptions {
                v8_platform: platform,
                module_loader: Some(loader),
                extensions: vec![ext],
                will_snapshot: false,
                inspector: true,
                ..Default::default()
            });

            *g_platform = true;
        }
    }
}

/// used for loading js modules
struct DBModuleLoader {}

impl ModuleLoader for DBModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, Error> {
        let url_str = format!("http://nino.db/{}", specifier);
        let url = Url::parse(&url_str)?;
        Ok(url)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<ModuleSpecifier>,
        _is_dyn_import: bool,
    ) -> Pin<Box<ModuleSourceFuture>> {
        let module_specifier = module_specifier.clone();
        async move {
            // generic_error(format!(
            //     "Provided module specifier \"{}\" was not found",
            //     module_specifier
            // ))
            println!("load module: {}", module_specifier.path());
            let code = r#"
            export default async function servlet(request, response) {
                Deno.core.print('js_servlet request: ' + JSON.stringify(request) + '\n');
                response.set('Content-Type', 'text/html;charset=UTF-8');
                await response.send('<hr/>method: ' + request.method + '<br/>path: ' + request.path + '</hr/>');
                return 42;
            }
            "#;

            let module_type = ModuleType::JavaScript;
            // ModuleType::Json

            let codebytes = code.as_bytes().to_vec().into_boxed_slice();

            let module = ModuleSource {
                code: codebytes,
                module_type,
                module_url_specified: module_specifier.to_string(),
                module_url_found: module_specifier.to_string(),
            };
            Ok(module)
        }
        .boxed_local()
    }
}

pub struct Task {
    pub id: u32,
}

fn create_state(state: &mut OpState) -> Result<(), Error> {
    state.put(Task { id: 0 });
    Ok(())
}

pub enum RetrievedV8Value<'s> {
    Value(v8::Local<'s, v8::Value>),
    Error(v8::Local<'s, v8::Value>),
    Promise(v8::Local<'s, v8::Promise>),
}

// This is done as a macro so that Rust can reuse the borrow on the scope,
// instead of treating the returned value's reference to the scope as a new mutable borrow.
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

pub fn run_deno_thread(
    cx: &mut Context,
    module_loader: Rc<dyn ModuleLoader>,
    get_ops: fn() -> Vec<OpDecl>,
    create_state: fn(state: &mut OpState) -> Result<(), Error>,
    javascript_source_code: &str,
    inspector_session_sx: Option<InspectorSessionProxy>,
    dbg_ready: Option<Sender<bool>>,
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

    // define inspector for the first instance
    if need_inspector {
        let debug_session = inspector_session_sx.unwrap();
        runtime.maybe_init_inspector();
        let inspector_rc = runtime.inspector();
        //let inspector = inspector_rc.borrow_mut();
        //let session_sender = inspector.get_session_sender();
        // let deregister_rx = inspector.add_deregister_handler();
        //session_sender.unbounded_send(debug_session).unwrap();
        //notify debug session
        let ready_s = dbg_ready.unwrap();
        ready_s.send(true).unwrap();
    }
    let result = runtime.execute_script("_main", javascript_source_code)?;
    {
        let mut scope = runtime.handle_scope();
        let local = deno_core::v8::Local::new(&mut scope, &result);
        let result = extract_promise!(&mut scope, local);
        match result {
            RetrievedV8Value::Value(v) => {
                return Poll::Ready(serde_v8::from_v8(&mut scope, v).map_err(Error::from))
            }
            RetrievedV8Value::Error(e) => {
                let js_error = deno_core::error::JsError::from_v8_exception(&mut scope, e);
                return Poll::Ready(Err(Error::msg(js_error.message.unwrap())));
            }
            RetrievedV8Value::Promise(_) => {
                // Wait for the promise to resolve.
            }
        };
    }
    let r = runtime.poll_event_loop(cx, need_inspector);
    eprintln!("js thread done");
    r
}
