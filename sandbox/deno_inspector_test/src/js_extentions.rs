use anyhow::Error;
use deno_core::*;
use deno_error::JsErrorBox;

pub const MODULE_MAIN: &str = "main";
const TEST_MAIN_MODULE_SOURCE: &str = include_str!("main.js");

pub async fn load_module_async(module_name: String) -> Result<String, Error> {
    println!("load module: {}", module_name);
    let code = if MODULE_MAIN == module_name {
        TEST_MAIN_MODULE_SOURCE
    } else {
        "export default async function() { return 'b'; }"
    };
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    Ok(code.to_string())
}

pub fn state_fn_0(state: &mut OpState) {
    state.put(TestTask {
        id: 0,
        str: "".to_string(),
    });
}

pub fn state_fn_1(state: &mut OpState) {
    state.put(TestTask {
        id: 1,
        str: "".to_string(),
    });
}

deno_core::extension!(
    nino_extention0,
    ops = [op_sum, test_a_sleep, test_sync, test_set_result, test_id],
    state = state_fn_0,
);
deno_core::extension!(
    nino_extention1,
    ops = [op_sum, test_a_sleep, test_sync, test_set_result, test_id],
    state = state_fn_1,
);

struct TestTask {
    id: u32,
    str: String,
}

#[op2]
fn op_sum(#[serde] nums: Vec<f64>) -> Result<f64, JsErrorBox> {
    // Sum inputs
    let sum = nums.iter().fold(0.0, |a, v| a + v);
    // return as a Result<f64, OpError>
    Ok(sum)
}

#[op2(async)]
async fn test_a_sleep(#[smi] millis: u64) -> Result<i32, JsErrorBox> {
    println!("waiting {} ms", millis);
    tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
    Ok(42)
}

#[op2]
#[string]
fn test_sync() -> String {
    String::from("OK")
}

#[op2(fast)]
fn test_set_result(state: &mut OpState, #[string] result: String) {
    {
        let test_state = state.borrow_mut::<TestTask>();
        let s = test_state.str.clone();
        println!("old res: {}", s);
        test_state.str = result.clone();
    }
}

#[op2(fast)]
fn test_id(state: &mut OpState) -> Result<u32, JsErrorBox> {
    let v;
    {
        let test_state = state.borrow_mut::<TestTask>();
        v = test_state.id;
        println!("[{}] sync get", v);
    }
    Ok(v)
}
