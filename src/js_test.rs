#[cfg(test)]
mod tests {

    use crate::js::{init_platform, run_deno_thread};
    use deno_core::{
        anyhow::Error, futures::FutureExt, op, ModuleLoader, ModuleSource, ModuleSourceFuture,
        ModuleSpecifier, ModuleType, OpDecl, OpState,
    };
    use http_types::Url;
    use std::{pin::Pin, rc::Rc, sync::Mutex};

    struct TestModuleLoader;

    impl ModuleLoader for TestModuleLoader {
        fn resolve(
            &self,
            specifier: &str,
            referrer: &str,
            kind: deno_core::ResolutionKind,
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
                println!("load module: {}", module_specifier.path());
                let code = "export default function() { return '42'; }";

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

    struct TestTask {
        id: u32,
    }

    fn get_ops() -> Vec<OpDecl> {
        vec![op_sync::decl(), op_id::decl(), op_set_result::decl()]
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
    fn op_id(op_state: &mut OpState) -> Result<u32, Error> {
        let v = 0;
        {
            //let test_state: &mut js::tests::Task = state.borrow_mut();
            //v = test_state.id;
        }
        //let r = format!("{}", v);
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
            const modValue = mod.default();
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
                        Ok(())
                    },
                    TEST_MAIN_MODULE_SOURCE,
                    None,
                    None,
                )
            }),
            poll_fn(|cx| {
                run_deno_thread(
                    cx,
                    Rc::new(TestModuleLoader {}),
                    get_ops,
                    |state| {
                        state.put(TestTask { id: 1 });
                        Ok(())
                    },
                    TEST_MAIN_MODULE_SOURCE,
                    None,
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
