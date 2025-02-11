// Print helper function, calling Deno.core.print()
function print(value) {
    Deno.core.print(value.toString() + "\n");
}

async function main() {
    const core = Deno.core;
    const arr = [1, 2, 3];
    print("The sum of");
    print(arr);
    print("is");
    print(core.ops.op_sum(arr));

    // And incorrect usage
    try {
        print(Deno.core.ops.op_sum(0));
    } catch (e) {
        print('Exception:');
        print(e);
    }

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


    var ever = 5;
    for (;ever;) {
        debugger;
        await Deno.core.ops.test_a_sleep(2000);
        print('waiting debugger');
        ever--;
    }
}
(async () => {
    await main();
})(); 