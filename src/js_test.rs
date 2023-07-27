#[cfg(test)]
mod tests {

    use crate::js::{init_platform, run_deno_thread};
    use deno_core::{
        anyhow::Error, futures::FutureExt, op, FastString, ModuleLoader, ModuleSource,
        ModuleSourceFuture, ModuleSpecifier, ModuleType, Op, OpDecl, OpState, ResolutionKind,
    };
    use http_types::Url;
    use std::{pin::Pin, rc::Rc, sync::Mutex};

    struct TestModuleLoader;
    const MODULE_URI: &str = "http://nino.db/";
    const MODULE_MAIN: &str = "main";

    impl ModuleLoader for TestModuleLoader {
        fn resolve(
            &self,
            specifier: &str,
            _referrer: &str,
            _kind: ResolutionKind,
        ) -> Result<ModuleSpecifier, Error> {
            let url;
            if specifier.starts_with(MODULE_URI) {
                url = Url::parse(&specifier)?;
            } else {
                let url_str = format!("{}{}", MODULE_URI, specifier);
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
            let module_specifier = module_specifier.clone();
            async move {
                // generic_error(format!(
                //     "Provided module specifier \"{}\" is not a file URL.",
                //     module_specifier
                // ))
                let module_path = &module_specifier.path()[1..];
                println!("load module: {}", module_path);
                let code;
                if MODULE_MAIN == module_path {
                    code = TEST_MAIN_MODULE_SOURCE;
                } else {
                    code = "export default async function() { return 42; }";
                }

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

    struct TestTask {
        id: u32,
    }

    fn get_ops() -> Vec<OpDecl> {
        vec![op_sync::DECL, op_id::DECL, op_set_result::DECL]
    }

    #[op]
    fn op_sync() -> Result<String, Error> {
        Ok(String::from("OK"))
    }

    #[op]
    fn op_set_result(result: String) -> Result<(), Error> {
        {
            let mut res = TEST_RESULTS.lock().unwrap();
            let v = res.as_mut().unwrap();
            println!("old res: {}", v);
            *res = Some(Box::new(result.clone()));
        }
        Ok(())
    }

    #[op]
    fn op_id(state: &mut OpState) -> Result<u32, Error> {
        let v;
        {
            let test_state = state.borrow_mut::<TestTask>();
            v = test_state.id;
            println!("[{}] sync get", v);
        }
        Ok(v)
    }

    static TEST_MAIN_MODULE_SOURCE: &'static str = r#"
    async function main() {
        let result = "";
        try{
            Deno.core.print('-------------------------\ntry\n');
            const id = Deno.core.ops.op_id();
            Deno.core.print('id ' + id + '\n');
            const value = Deno.core.ops.op_sync();
            Deno.core.print('value ' + value + '\n');
            const mod = await import("b");
            const modValue = await mod.default();
            Deno.core.print('modValue ' + modValue + '\n');
            result = '' + id + value + modValue;
        }catch(e){
            result = ' error: ' + e;
        }
        Deno.core.print('RESULT: ' + result + '\n');
        Deno.core.ops.op_set_result(result);
    }
    (async () => { 
        await main();
    })();
    "#;

    static TEST_RESULTS: Mutex<Option<Box<String>>> = Mutex::new(None);

    async fn test_js() {
        init_platform(2);
        // second call should not matter
        init_platform(2);
        {
            let mut results = TEST_RESULTS.lock().unwrap();
            *results = Some(Box::new(String::new()));
        }

        let r = tokio::try_join!(
            poll_fn(|cx| {
                run_deno_thread(
                    cx,
                    Rc::new(TestModuleLoader {}),
                    get_ops,
                    |state| {
                        state.put(TestTask { id: 0 });
                        ()
                    },
                    TEST_MAIN_MODULE_SOURCE,
                    None,
                )
            }),
            poll_fn(|cx| {
                run_deno_thread(
                    cx,
                    Rc::new(TestModuleLoader {}),
                    get_ops,
                    |state| {
                        state.put(TestTask { id: 0 });
                        ()
                    },
                    TEST_MAIN_MODULE_SOURCE,
                    None,
                )
            }),
        );
        match r {
            Err(e) => {
                panic!("JS ERROR: {}", e.to_string());
            }
            Ok(_v) => {
                let mut res = TEST_RESULTS.lock().unwrap();
                let str = (*res).as_mut().unwrap().as_mut();
                println!("result: {}", str);
                assert_eq!(*str, "0OK42");
            }
        };
    }

    #[test]
    fn deno_simple_test() {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .unwrap()
            .block_on(async { test_js().await });
    }
}
