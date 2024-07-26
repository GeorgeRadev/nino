#[cfg(test)]
mod tests {
    use crate::js_worker::{MainWorker, WorkerOptions};
    
    use deno_core::{
        self, anyhow::Error, futures::FutureExt, op2, FastString, ModuleLoadResponse, ModuleLoader,
        ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType, OpState, RequestedModuleType,
        ResolutionKind,
    };
    use reqwest::Url;
    use deno_runtime::inspector_server::InspectorServer;
    use deno_runtime::errors;
    use std::net::SocketAddr;
    use std::sync::Arc;
    use std::{cell::RefCell, rc::Rc};

    struct TestState {
        id: i32,
    }

    #[op2(fast)]
    fn test_set(state: &mut OpState, v: i32) -> Result<i32, Error> {
        let test_state = state.borrow_mut::<TestState>();
        test_state.id = v;
        println!("[{}] sync set", v);
        Ok(v)
        //Ok(String::from("Test"))
    }

    #[op2(fast)]
    fn test_get(state: &mut OpState) -> Result<i32, Error> {
        let v;
        {
            let test_state = state.borrow_mut::<TestState>();
            v = test_state.id;
            println!("[{}] sync get", v);
        }
        Ok(v)
    }

    #[op2(async)]
    async fn test_a_get(state: Rc<RefCell<OpState>>) -> Result<i32, Error> {
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

    #[op2(async)]
    async fn test_a_sleep(state: Rc<RefCell<OpState>>, #[smi] millis: u64) -> Result<i32, Error> {
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

    struct ModsLoader;
    const MODULE_URI: &str = "http://nino.db/";
    const MODULE_MAIN: &str = "main";
    const TEST_MAIN_MODULE_SOURCE: &str = r#"
    async function main() {
        const core = Deno.core;
    
        try {
            core.print('-- start\n');
    
            let ever = false;
            for (; ever;) {
                // sleep for a second
                await core.ops.test_a_sleep(1000);
                debugger;
            }
    
            let id = core.ops.test_get();
            core.print('-- after ops.test_get ' + id + '\n');
    
            id = await core.ops.test_a_get();
            core.print('-- after opAsync test_a_get ' + id + '\n');
    
            id = await core.ops.test_a_sleep(1);
            core.print('-- after opAsync test_a_sleep\n');
    
            let m = await import('b');
            core.print('module keys: ' + Object.keys(m) + '\n');
            core.print('module type: ' + typeof m + '\n');
            core.print('module default type: ' + typeof m.default + '\n');
            core.print('module await default(): ' + (await m.default()) + '\n');
    
            id = await core.ops.test_a_sleep(2);
            core.print('-- after opAsync test_a_sleep ' + id + '\n');
    
            id = core.ops.test_get();
            core.print('-- after ops.test_get ' + id + '\n');
    
            core.print('-- done !! OK\n');
        } catch (e) {
            core.print(' error: ' + e + '\n');
        }
    }
    (async () => {
        await main();
    })(); 
    "#;

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
            _maybe_referrer: Option<&ModuleSpecifier>,
            _is_dyn_import: bool,
            _requested_module_type: RequestedModuleType,
        ) -> ModuleLoadResponse {
            let module_specifier = module_specifier.clone();
            let res = async move {
                let module_path = &module_specifier.path()[1..];
                println!("load module: {}", module_path);
                let code = if MODULE_MAIN == module_path {
                    TEST_MAIN_MODULE_SOURCE
                } else {
                    "export default async function() { return 'b'; }"
                };

                let module_type = ModuleType::JavaScript;
                let code = ModuleSourceCode::String(FastString::from(String::from(code)));
                let module_string = module_specifier.clone();
                let module = ModuleSource::new(module_type, code, &module_string, None);
                Ok(module)
            }
            .boxed_local();
            ModuleLoadResponse::Async(res)
        }
    }

    fn get_error_class_name(e: &Error) -> &'static str {
        errors::get_error_class_name(e).unwrap_or("Error")
    }

    async fn test_debugger() -> Result<(), Error> {
        //    init_platform(2);
        let module_loader = Rc::new(ModsLoader {});

        deno_core::extension!(
            extension,
            ops = [test_get, test_set, test_a_get, test_a_sleep],
            state = |state| {
                state.put(TestState { id: 0 });
            },
        );

        let extensions = vec![extension::init_ops()];

        let inspector_address = "127.0.0.1:9229".parse::<SocketAddr>().unwrap();
        let inspector_server = Arc::new(InspectorServer::new(inspector_address, "nino")?);

        let options = WorkerOptions {
            extensions,
            startup_snapshot: None,
            source_map_getter: None,
            module_loader,
            get_error_class_fn: Some(&get_error_class_name),
            shared_array_buffer_store: None,
            compiled_wasm_module_store: None,
            maybe_inspector_server: Some(inspector_server.clone()),
            should_break_on_first_statement: false,
            should_wait_for_inspector_session: false,
            ..Default::default()
        };

        let main_uri = format!("{}{}", MODULE_URI, MODULE_MAIN).to_owned();
        let main_module = Url::parse(main_uri.as_str())?;

        let mut worker = MainWorker::from_options(main_module.clone(), options);
        worker.execute_main_module(&main_module).await?;
        worker.run_event_loop(false).await?;

        drop(inspector_server);
        Ok(())
    }

    #[test]
    fn deno_simple_debugger() {
        let _r = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { test_debugger().await });
    }
}
