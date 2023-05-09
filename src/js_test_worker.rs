#[cfg(test)]
mod tests {
    use crate::js::{init_platform, run_deno_main_thread};
    use deno_core::futures::FutureExt;
    use deno_core::{anyhow::Error, op, OpDecl, OpState};
    use std::{future::Future, sync::Mutex};
    use std::pin::Pin;

    const MODULE_MAIN: &str = "main";

    static TEST_MAIN_MODULE_SOURCE: &'static str = r#"
    async function main() {
        const core = Deno[Deno.internal].core;
        let result = "";
        try{
            core.print('-------------------------\ntry\n');
            const id = core.ops.op_id();
            core.print('id ' + id + '\n');
            const value = core.ops.op_sync();
            core.print('value ' + value + '\n');
            const mod = await import("b");
            const modValue = await mod.default();
            core.print('modValue ' + modValue + '\n');
            result = '' + id + value + modValue;
        }catch(e){
            result = ' error: ' + e;
        }
        core.print('RESULT: ' + result + '\n');
        core.ops.op_set_result(result);
    }
    (async () => { 
        await main();
    })();
    "#;

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
    fn op_id(state: &mut OpState) -> Result<u32, Error> {
        let v;
        {
            let test_state = state.borrow_mut::<TestTask>();
            v = test_state.id;
            println!("[{}] sync get", v);
        }
        Ok(v)
    }

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
            async {
                run_deno_main_thread(
                    module_loader,
                    get_ops,
                    |state| {
                        state.put(TestTask { id: 1 });
                    },
                    "main",
                    9229,
                )
                .await
            },
            async {
                run_deno_main_thread(
                    module_loader,
                    get_ops,
                    |state| {
                        state.put(TestTask { id: 1 });
                    },
                    "main",
                    0,
                )
                .await
            },
        );
        match r {
            Err(e) => {
                panic!("JS ERROR: {}", e.to_string());
            }
            Ok(_v) => {
                let mut res = TEST_RESULTS.lock().unwrap();
                let str = (*res).as_mut().unwrap().as_mut();
                println!("result: {}", str);
                assert_eq!(*str, "1OK42");
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
