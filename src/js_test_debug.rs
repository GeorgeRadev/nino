#[cfg(test)]
mod tests {
    use deno_core::anyhow::Error;
    use deno_core::error::AnyError;
    use deno_core::futures::FutureExt;
    use deno_core::*;
    use deno_runtime::deno_broadcast_channel::InMemoryBroadcastChannel;
    use deno_runtime::inspector_server::InspectorServer;
    use deno_runtime::permissions::PermissionsContainer;
    use deno_runtime::worker::MainWorker;
    use deno_runtime::worker::WorkerOptions;
    use deno_runtime::BootstrapOptions;
    use http_types::Url;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::{cell::RefCell, pin::Pin, rc::Rc};

    struct TestState {
        id: i32,
    }

    fn create_state(state: &mut OpState) {
        state.put(TestState { id: 0 });
    }

    #[op]
    fn op_set(state: &mut OpState, v: i32) -> Result<i32, AnyError> {
        let test_state = state.borrow_mut::<TestState>();
        test_state.id = v;
        println!("[{}] sync set", v);
        Ok(v)
        //Ok(String::from("Test"))
    }

    #[op]
    fn op_get(state: &mut OpState) -> Result<i32, AnyError> {
        let v;
        {
            let test_state = state.borrow_mut::<TestState>();
            v = test_state.id;
            println!("[{}] sync get", v);
        }
        Ok(v)
    }

    #[op]
    async fn op_async(state: Rc<RefCell<OpState>>) -> Result<i32, AnyError> {
        let v;
        {
            let mut op_state = state.borrow_mut();
            let test_state = op_state.borrow_mut::<TestState>();
            v = test_state.id;
        }
        println!("[{}] async get", v);
        // you need to await something in the async function
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        Ok(v)
    }

    #[op]
    async fn op_a_sleep(state: Rc<RefCell<OpState>>, millis: u64) -> Result<i32, AnyError> {
        let v;
        {
            let mut op_state = state.borrow_mut();
            let test_state = op_state.borrow_mut::<TestState>();
            v = test_state.id;
        }
        println!("[{}] waiting {} ms", v, millis);
        tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
        Ok(v)
    }

    fn get_extensions() -> Vec<Extension> {
        let ext = Extension::builder("nino_extentions")
            .ops(vec![
                op_get::DECL,
                op_set::DECL,
                op_async::DECL,
                op_a_sleep::DECL,
            ])
            .state(create_state)
            //.force_op_registration()
            .build();
        vec![ext]
    }

    static TEST_MAIN_MODULE_SOURCE: &str = r#"
    async function main() {
        const core = Deno[Deno.internal].core;
        try {
            core.print('-- start\n');
    
            core.print('-- waiting for debugger \n');
            let ever = false;
            for (; ever;) {
                // sleep for a second
                await core.opAsync('op_a_sleep', 1000);
                debugger;
            }
    
            let id = core.ops.op_get();
            core.print('-- after ops.op_get ' + id + '\n');
    
            id = await core.opAsync('op_async');
            core.print('-- after opAsync op_async ' + id + '\n');
    
            id = await core.opAsync('op_a_sleep', 1);
            core.print('-- after opAsync op_a_sleep\n');
    
            let m = await import('b');
            core.print('module keys: ' + Object.keys(m) + '\n');
            core.print('module type: ' + typeof m + '\n');
            core.print('module default type: ' + typeof m.default + '\n');
            core.print('module await default(): ' + (await m.default()) + '\n');
    
            id = await core.opAsync('op_a_sleep', 2);
            core.print('-- after opAsync op_a_sleep ' + id + '\n');
    
            id = core.ops.op_get();
            core.print('-- after ops.op_get ' + id + '\n');
    
            core.print('-- done !! OK\n');
        } catch (e) {
            core.print(' error: ' + e + '\n');
        }
    }
    (async () => {
        await main();
    })();
    "#;

    struct ModsLoader;
    const MODULE_URI: &str = "http://nino.db/";
    const MODULE_MAIN: &str = "main";

    impl ModuleLoader for ModsLoader {
        fn resolve(
            &self,
            specifier: &str,
            _referrer: &str,
            _kind: ResolutionKind,
        ) -> Result<ModuleSpecifier, Error> {
            let url = if specifier.starts_with(MODULE_URI) {
                Url::parse(specifier)?
            } else {
                let url_str = format!("{}{}", MODULE_URI, specifier);
                Url::parse(&url_str)?
            };
            Ok(url)
        }

        fn load(
            &self,
            module_specifier: &ModuleSpecifier,
            _maybe_referrer: std::option::Option<&deno_core::url::Url>,
            _is_dyn_import: bool,
        ) -> Pin<Box<ModuleSourceFuture>> {
            let module_specifier = module_specifier.clone();
            async move {
                // generic_error(format!(
                //     "Provided module specifier \"{}\" is not a file URL.",
                //     module_specifier
                // ))
                let module_path = &module_specifier.path()[1..];
                println!("load module: {}", module_path);
                let code = if MODULE_MAIN == module_path {
                    TEST_MAIN_MODULE_SOURCE
                } else {
                    "export default async function() { return 'b'; }"
                };

                let module_type = ModuleType::JavaScript;
                // ModuleType::Json
                let code = FastString::from(String::from(code)); //code.as_bytes().to_vec().into_boxed_slice();
                let module_string = module_specifier.clone();
                let module = ModuleSource::new(module_type, code, &module_string);
                Ok(module)
            }
            .boxed_local()
        }
    }

    fn get_error_class_name(e: &AnyError) -> &'static str {
        deno_runtime::errors::get_error_class_name(e).unwrap_or("Error")
    }

    async fn test_debugger() -> Result<(), AnyError> {
        //    init_platform(2);

        let module_loader = Rc::new(ModsLoader {});

        let create_web_worker_cb = Arc::new(|_| {
            todo!("Web workers are not supported in the example");
        });
        let web_worker_event_cb = Arc::new(|_| {
            todo!("Web workers are not supported in the example");
        });

        let extensions = get_extensions();

        let inspector_address = "127.0.0.1:9229".parse::<SocketAddr>().unwrap();
        let inspector_server = Arc::new(InspectorServer::new(inspector_address, "nino"));

        let options = WorkerOptions {
            bootstrap: BootstrapOptions::default(),
            extensions,
            startup_snapshot: None,
            unsafely_ignore_certificate_errors: None,
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
            broadcast_channel: InMemoryBroadcastChannel::default(),
            shared_array_buffer_store: None,
            compiled_wasm_module_store: None,
            maybe_inspector_server: Some(inspector_server.clone()),
            should_break_on_first_statement: false,
            should_wait_for_inspector_session: false,
            ..Default::default()
        };

        let main_uri = format!("{}{}", MODULE_URI, MODULE_MAIN).to_owned();
        let main_module = Url::parse(main_uri.as_str())?;
        let permissions =
            PermissionsContainer::new(deno_runtime::permissions::Permissions::default());

        let mut worker =
            MainWorker::bootstrap_from_options(main_module.clone(), permissions, options);

        println!("Connect to debugger and change the loop valiable ever to false");
        worker.execute_main_module(&main_module).await?;
        worker.run_event_loop(false).await?;

        drop(inspector_server);
        Ok(())
    }

    #[test]
    fn deno_simple_debugger() {
        let _r = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .unwrap()
            .block_on(async { test_debugger().await });
    }
}
