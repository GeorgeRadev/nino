#[cfg(test)]
mod tests {
    use crate::js::{init_platform, run_deno_main_thread};
    use deno_runtime::deno_core::{self, Op, OpDecl};
    use deno_runtime::deno_core::{anyhow::Error, futures::FutureExt, op2, OpState};
    use std::pin::Pin;
    use std::{future::Future, sync::Mutex};

    fn module_loader(
        module_name: String,
    ) -> Pin<Box<dyn Future<Output = Result<String, Error>> + 'static>> {
        async move {
            // todo : implement DB load module
            if MODULE_MAIN == module_name {
                Ok(String::from(TEST_MAIN_MODULE_SOURCE))
            } else {
                Ok(String::from(
                    "export default async function() { return 42; }",
                ))
            }
        }
        .boxed_local()
    }

    static TEST_RESULTS: Mutex<Option<String>> = Mutex::new(None);
    struct TestTask {
        id: u32,
    }

    #[op2]
    #[string]
    fn test_sync(_state: &mut OpState) -> String {
        String::from("OK")
    }

    #[op2(fast)]
    fn test_set_result(#[string] result: String) {
        {
            let mut res = TEST_RESULTS.lock().unwrap();
            let v = res.as_mut().unwrap();
            println!("old res: {}", v);
            *res = Some(result.clone());
        }
    }

    #[op2(fast)]
    fn test_id(state: &mut OpState) -> Result<u32, Error> {
        let v;
        {
            let test_state = state.borrow_mut::<TestTask>();
            v = test_state.id;
            println!("[{}] sync get", v);
        }
        Ok(v)
    }

    fn ops() -> Vec<OpDecl> {
        vec![test_sync::DECL, test_id::DECL, test_set_result::DECL]
    }

    fn state_fn_0(state: &mut OpState) -> () {
        state.put(TestTask { id: 0 });
    }

    fn state_fn_1(state: &mut OpState) -> () {
        state.put(TestTask { id: 1 });
    }

    const MODULE_MAIN: &str = "main";

    static TEST_MAIN_MODULE_SOURCE: &str = r#"
    async function main() {
        const core = Deno.core;
        let result = "";
        try{
            core.print('-------------------------\ntry\n');
            const id = core.ops.test_id();
            core.print('id ' + id + '\n');
            const value = core.ops.test_sync();
            core.print('value ' + value + '\n');
            const mod = await import("b");
            const modValue = await mod.default();
            core.print('modValue ' + modValue + '\n');
            result = '' + id + value + modValue;
        }catch(e){
            result = ' error: ' + e;
        }
        core.print('RESULT: ' + result + '\n');
        core.ops.test_set_result(result);
    }
    (async () => { 
        await main();
    })();
    "#;

    async fn test_js() {
        // init platform
        init_platform(2, module_loader);
        // second call should not matter
        init_platform(2, module_loader);
        {
            let mut results = TEST_RESULTS.lock().unwrap();
            *results = Some(String::new());
        }

        // run two modules
        let r = tokio::try_join!(
            async {
                run_deno_main_thread(module_loader, ops(), state_fn_0, "main", None, 9339, false)
                    .await
            },
            async {
                run_deno_main_thread(module_loader, ops(), state_fn_0, "main", None, 0, false).await
            },
        );

        match r {
            Err(e) => {
                panic!("JS ERROR: {}", e);
            }
            Ok(_v) => {
                let mut res = TEST_RESULTS.lock().unwrap();
                let str = res.as_mut().unwrap();
                println!("result: {}", str);
                assert_eq!(str, "0OK42");
            }
        };

        {
            let mut results = TEST_RESULTS.lock().unwrap();
            *results = Some(String::new());
        }

        // run code
        if let Err(error) = run_deno_main_thread(
            module_loader,
            ops(),
            state_fn_1,
            "main",
            Some(TEST_MAIN_MODULE_SOURCE),
            0,
            false,
        )
        .await
        {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            assert_eq!(0, 1);
        } else {
            let mut res = TEST_RESULTS.lock().unwrap();
            let str = res.as_mut().unwrap();
            println!("result: {}", str);
            assert_eq!(str, "1OK42");
        }
    }

    #[test]
    fn deno_simple_test() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { test_js().await });
    }
}
