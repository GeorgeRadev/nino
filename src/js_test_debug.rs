#[cfg(test)]
mod tests {
    use crate::js::{init_platform, run_deno_thread};
    use deno_core::{
        anyhow::Error,
        futures::{
            channel::{
                mpsc::{self, UnboundedReceiver, UnboundedSender},
                oneshot::{self, Receiver},
            },
            FutureExt, SinkExt,
        },
        op, InspectorMsg, InspectorSessionProxy, ModuleLoader, ModuleSource, ModuleSourceFuture,
        ModuleSpecifier, ModuleType, OpDecl, OpState,
    };
    use http_types::Url;
    use std::{pin::Pin, rc::Rc, sync::Mutex};
    use core::task::Poll;
    use tokio::macros::support::poll_fn;
    use tokio::task;

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
        debugger;
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

    async fn debugger(
        mut dbg_receiver: Receiver<bool>,
        mut inbound_tx: UnboundedSender<String>,
        mut outbound_rx: UnboundedReceiver<InspectorMsg>,
    ) -> Result<(), Error> {
        // wait for rx element to simulate join
        loop {
            if let Ok(_) = dbg_receiver.try_recv() {
                break;
            }
        }

        //init debugger
        inbound_tx
            .send(r#"{"id":1,"method":"Runtime.enable"}"#.to_string())
            .await
            .unwrap();
        inbound_tx
            .send(r#"{"id":2,"method":"Debugger.enable"}"#.to_string())
            .await
            .unwrap();
        inbound_tx
        .send(r#"{"id":4,"method":"Runtime.evaluate","params":{"expression":"Deno.core.print(\"hello from the inspector\\n\")","contextId":1,"includeCommandLineAPI":true,"silent":false,"returnByValue":true}}"#.to_string())
        .await
        .unwrap();
        // wait for first message
        loop {
            match outbound_rx.try_next() {
                Ok(msg) => match msg {
                    Some(msg) => {
                        println!("inspector: {}", msg.content);
                        break;
                    }
                    None => {
                        task::yield_now().await;
                    }
                },
                Err(_) => {}
            }
        }
        // loop untill disconnected
        loop {
            match outbound_rx.try_next() {
                Ok(msg) => match msg {
                    Some(msg) => {
                        println!("inspector: {}", msg.content);
                    }
                    None => {
                        tokio::task::yield_now().await;
                    }
                },
                Err(error) => {
                    println!("inspector ERROR: {}", error);
                    break;
                }
            }
        }
        Ok(())
    }

    async fn run_deno_thread_with_debugger(
        outbound_sx: UnboundedSender<InspectorMsg>, 
        inbound_rx:UnboundedReceiver<String>, 
        dbg_ready_sx: oneshot::Sender<bool> ) -> Poll<Result<(), Error>> {
        let deno_closure = |cx| {

        let inspector_session_proxy = InspectorSessionProxy {
            tx: outbound_sx,
            rx: inbound_rx,
        };
        
            run_deno_thread(
                cx,
                Rc::new(TestModuleLoader {}),
                get_ops,
                |state| {
                    state.put(TestTask { id: 0 });
                    Ok(())
                },
                TEST_MAIN_MODULE_SOURCE,
                Some(inspector_session_proxy),
                Some(dbg_ready_sx),
            )
        };
        Poll::Ready(poll_fn(|cx| deno_closure(cx)).await)
    }

    async fn test_debugger() {
        init_platform(2);
        // The 'inbound' channel carries messages send to the inspector.
        let (inbound_sx, inbound_rx) = mpsc::unbounded();
        // The 'outbound' channel carries messages received from the inspector.
        let (outbound_sx, outbound_rx) = mpsc::unbounded();
        // use oneshot as signal for inspector initialized
        let (dbg_ready_sx, dbg_ready_rx) = oneshot::channel::<bool>();
 
        let _r = tokio::try_join!(
            run_deno_thread_with_debugger(outbound_sx, inbound_rx, dbg_ready_sx),
            debugger(dbg_ready_rx, inbound_sx, outbound_rx),
        );
        println!("Done");
    }

    #[test]
    fn deno_simple_debugger() {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
            .unwrap()
            .block_on(async { test_debugger().await });
    }
}
