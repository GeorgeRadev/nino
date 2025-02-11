use std::future::Future;
use std::pin::Pin;

use deno_core::anyhow::Error;
use deno_core::futures::FutureExt;
use deno_core::url::Url;
use deno_core::Extension;
use js_core::ExecuteMode;
use js_extentions::nino_extention0;
use js_extentions::nino_extention1;

mod js_core;
mod js_extentions;
mod js_inspector;

fn load_module(
    module_name: String,
) -> Pin<Box<dyn Future<Output = Result<String, Error>> + 'static>> {
    js_extentions::load_module_async(module_name).boxed_local()
}

pub fn load_extentions0() -> Vec<Extension> {
    vec![nino_extention0::init_ops()]
}

pub fn load_extentions1() -> Vec<Extension> {
    vec![nino_extention1::init_ops()]
}

fn main() -> Result<(), Error> {
    js_core::js_init(load_module, 4);
    let main_uri = format!("{}{}", js_core::MODULE_URI, js_extentions::MODULE_MAIN).to_owned();
    let main_module = Url::parse(main_uri.as_str())?;

    println!("staring js... w/ Debugger...");
    let thread1 = js_core::start_js_thread(
        load_extentions0,
        ExecuteMode::JsModule(main_module.clone()),
        false,
        9229,
    )?;

    println!("staring js...w/o Debugger...");
    let thread2 = js_core::start_js_thread(
        load_extentions1,
        ExecuteMode::JsModule(main_module.clone()),
        false,
        0,
    )?;

    let code = r#"Deno.core.print('!!!!!!!from the code!!!!!!!!\n');"#.to_string();
    let thread3 = js_core::start_js_thread(load_extentions1, ExecuteMode::JsCode(code), false, 0)?;

    let _ = thread1.join();
    println!("...DONE");
    let _ = thread2.join();
    println!("...DONE");
    let _ = thread3.join();
    println!("...DONE");

    Ok(())
}
