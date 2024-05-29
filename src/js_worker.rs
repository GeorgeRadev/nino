/*
This worker is the same as the Deno Worker but using only the InspectorServer and jsRuntime, i.e. no other extentions.
*/
use deno_runtime::deno_broadcast_channel::InMemoryBroadcastChannel;
use deno_runtime::deno_core::{
    error::AnyError, error::JsError, v8, CompiledWasmModuleStore, Extension, FeatureChecker,
    FsModuleLoader, GetErrorClassFn, JsRuntime, ModuleCodeString, ModuleId, ModuleLoader,
    ModuleSpecifier, RuntimeOptions, SharedArrayBufferStore, SourceMapGetter,
};
use deno_runtime::{
    deno_io::Stdio, deno_tls::RootCertStoreProvider, deno_web::BlobStore,
    inspector_server::InspectorServer, BootstrapOptions,
};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub type FormatJsErrorFn = dyn Fn(&JsError) -> String + Sync + Send;

pub fn import_meta_resolve_callback(
    loader: &dyn deno_runtime::deno_core::ModuleLoader,
    specifier: String,
    referrer: String,
) -> Result<ModuleSpecifier, AnyError> {
    loader.resolve(
        &specifier,
        &referrer,
        deno_runtime::deno_core::ResolutionKind::DynamicImport,
    )
}

// TODO(bartlomieju): temporary measurement until we start supporting more
// module types
pub fn validate_import_attributes_callback(
    scope: &mut v8::HandleScope,
    attributes: &HashMap<String, String>,
) {
    for (key, value) in attributes {
        let msg = if key != "type" {
            Some(format!("\"{key}\" attribute is not supported."))
        } else if value != "json" {
            Some(format!("\"{value}\" is not a valid module type."))
        } else {
            None
        };

        let Some(msg) = msg else {
            continue;
        };

        let message = v8::String::new(scope, &msg).unwrap();
        let exception = v8::Exception::type_error(scope, message);
        scope.throw_exception(exception);
        return;
    }
}

/// This worker is created and used by almost all
/// subcommands in Deno executable.
///
/// It provides ops available in the `Deno` namespace.
///
/// All `WebWorker`s created during program execution
/// are descendants of this worker.
pub struct MainWorker {
    pub js_runtime: JsRuntime,
    should_break_on_first_statement: bool,
    should_wait_for_inspector_session: bool,
}

pub struct WorkerOptions {
    pub bootstrap: BootstrapOptions,

    /// JsRuntime extensions, not to be confused with ES modules.
    ///
    /// Extensions register "ops" and JavaScript sources provided in `js` or `esm`
    /// configuration. If you are using a snapshot, then extensions shouldn't
    /// provide JavaScript sources that were already snapshotted.
    pub extensions: Vec<Extension>,

    /// V8 snapshot that should be loaded on startup.
  pub startup_snapshot: Option<&'static [u8]>,

    /// Should op registration be skipped?
    pub skip_op_registration: bool,

    /// Optional isolate creation parameters, such as heap limits.
    pub create_params: Option<v8::CreateParams>,

    pub unsafely_ignore_certificate_errors: Option<Vec<String>>,
    pub root_cert_store_provider: Option<Arc<dyn RootCertStoreProvider>>,
    pub seed: Option<u64>,

    /// Implementation of `ModuleLoader` which will be
    /// called when V8 requests to load ES modules.
    ///
    /// If not provided runtime will error if code being
    /// executed tries to load modules.
    pub module_loader: Rc<dyn ModuleLoader>,
    // Callbacks invoked when creating new instance of WebWorker
    pub format_js_error_fn: Option<Arc<FormatJsErrorFn>>,

    /// Source map reference for errors.
    pub source_map_getter: Option<Rc<dyn SourceMapGetter>>,
    pub maybe_inspector_server: Option<Arc<InspectorServer>>,
    // If true, the worker will wait for inspector session and break on first
    // statement of user code. Takes higher precedence than
    // `should_wait_for_inspector_session`.
    pub should_break_on_first_statement: bool,
    // If true, the worker will wait for inspector session before executing
    // user code.
    pub should_wait_for_inspector_session: bool,
    /// If Some, print a low-level trace output for ops matching the given patterns.
    pub strace_ops: Option<Vec<String>>,

    /// Allows to map error type to a string "class" used to represent
    /// error in JavaScript.
    pub get_error_class_fn: Option<GetErrorClassFn>,
    pub blob_store: Arc<BlobStore>,
    pub broadcast_channel: InMemoryBroadcastChannel,

    /// The store to use for transferring SharedArrayBuffers between isolates.
    /// If multiple isolates should have the possibility of sharing
    /// SharedArrayBuffers, they should use the same [SharedArrayBufferStore]. If
    /// no [SharedArrayBufferStore] is specified, SharedArrayBuffer can not be
    /// serialized.
    pub shared_array_buffer_store: Option<SharedArrayBufferStore>,

    /// The store to use for transferring `WebAssembly.Module` objects between
    /// isolates.
    /// If multiple isolates should have the possibility of sharing
    /// `WebAssembly.Module` objects, they should use the same
    /// [CompiledWasmModuleStore]. If no [CompiledWasmModuleStore] is specified,
    /// `WebAssembly.Module` objects cannot be serialized.
    pub compiled_wasm_module_store: Option<CompiledWasmModuleStore>,
    pub stdio: Stdio,
    pub feature_checker: Arc<FeatureChecker>,
}

impl Default for WorkerOptions {
    fn default() -> Self {
        Self {
            module_loader: Rc::new(FsModuleLoader),
            skip_op_registration: false,
            seed: None,
            unsafely_ignore_certificate_errors: Default::default(),
            should_break_on_first_statement: Default::default(),
            should_wait_for_inspector_session: Default::default(),
            strace_ops: Default::default(),
            compiled_wasm_module_store: Default::default(),
            shared_array_buffer_store: Default::default(),
            maybe_inspector_server: Default::default(),
            format_js_error_fn: Default::default(),
            get_error_class_fn: Default::default(),
            broadcast_channel: Default::default(),
            source_map_getter: Default::default(),
            root_cert_store_provider: Default::default(),
            blob_store: Default::default(),
            extensions: Default::default(),
            startup_snapshot: Default::default(),
            create_params: Default::default(),
            bootstrap: Default::default(),
            stdio: Default::default(),
            feature_checker: Default::default(),
        }
    }
}

impl MainWorker {
    pub fn from_options(main_module: ModuleSpecifier, mut options: WorkerOptions) -> Self {
        let mut extensions: Vec<Extension> = vec![];
        extensions.extend(std::mem::take(&mut options.extensions));

        let has_notified_of_inspector_disconnect = AtomicBool::new(false);
        let wait_for_inspector_disconnect_callback = Box::new(move || {
            if !has_notified_of_inspector_disconnect.swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                println!(
                    "Program finished. Waiting for inspector to disconnect to exit the process..."
                );
            }
        });

        let mut js_runtime = JsRuntime::new(RuntimeOptions {
            module_loader: Some(options.module_loader.clone()),
            startup_snapshot: options.startup_snapshot,
            create_params: options.create_params,
            source_map_getter: options.source_map_getter,
            skip_op_registration: options.skip_op_registration,
            get_error_class_fn: options.get_error_class_fn,
            shared_array_buffer_store: options.shared_array_buffer_store.clone(),
            compiled_wasm_module_store: options.compiled_wasm_module_store.clone(),
            extensions,
            inspector: options.maybe_inspector_server.is_some(),
            is_main: true,
            feature_checker: Some(options.feature_checker.clone()),
            wait_for_inspector_disconnect_callback: Some(wait_for_inspector_disconnect_callback),
            import_meta_resolve_callback: Some(Box::new(import_meta_resolve_callback)),
            validate_import_attributes_cb: Some(Box::new(validate_import_attributes_callback)),
            ..Default::default()
        });

        if let Some(server) = options.maybe_inspector_server.clone() {
            server.register_inspector(
                main_module.to_string(),
                &mut js_runtime,
                options.should_break_on_first_statement
                    || options.should_wait_for_inspector_session,
            );

            // Put inspector handle into the op state so we can put a breakpoint when
            // executing a CJS entrypoint.
            let op_state = js_runtime.op_state();
            let inspector = js_runtime.inspector();
            op_state.borrow_mut().put(inspector);
        }

        Self {
            js_runtime,
            should_break_on_first_statement: options.should_break_on_first_statement,
            should_wait_for_inspector_session: options.should_wait_for_inspector_session,
        }
    }

    /// See [JsRuntime::execute_script](deno_core::JsRuntime::execute_script)
    pub fn execute_script(
        &mut self,
        script_name: &'static str,
        source_code: ModuleCodeString,
    ) -> Result<v8::Global<v8::Value>, AnyError> {
        self.js_runtime.execute_script(script_name, source_code)
    }

    /// Loads and instantiates specified JavaScript module as "main" module.
    pub async fn preload_main_module(
        &mut self,
        module_specifier: &ModuleSpecifier,
    ) -> Result<ModuleId, AnyError> {
        self.js_runtime
        .load_main_es_module(module_specifier)
            .await
    }

    /// Executes specified JavaScript module.
    pub async fn evaluate_module(&mut self, id: ModuleId) -> Result<(), AnyError> {
        self.wait_for_inspector_session();
        let mut receiver = self.js_runtime.mod_evaluate(id);
        tokio::select! {
          // Not using biased mode leads to non-determinism for relatively simple
          // programs.
          biased;

          maybe_result = &mut receiver => {
            // println!("received module evaluate {:#?}", maybe_result);
            maybe_result
          }

          event_loop_result = self.run_event_loop(false) => {
            event_loop_result?;
            receiver.await
          }
        }
    }

    /// Loads, instantiates and executes specified JavaScript module.
    ///
    /// This module will have "import.meta.main" equal to true.
    pub async fn execute_main_module(
        &mut self,
        module_specifier: &ModuleSpecifier,
    ) -> Result<(), AnyError> {
        let id = self.preload_main_module(module_specifier).await?;
        self.evaluate_module(id).await
    }

    fn wait_for_inspector_session(&mut self) {
        if self.should_break_on_first_statement {
            self.js_runtime
                .inspector()
                .borrow_mut()
                .wait_for_session_and_break_on_next_statement();
        } else if self.should_wait_for_inspector_session {
            self.js_runtime.inspector().borrow_mut().wait_for_session();
        }
    }

    pub async fn run_event_loop(&mut self, wait_for_inspector: bool) -> Result<(), AnyError> {
        self.js_runtime
            .run_event_loop(deno_runtime::deno_core::PollEventLoopOptions {
                wait_for_inspector,
                ..Default::default()
            })
            .await
    }
}
