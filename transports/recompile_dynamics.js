async function main() {
    const core = Deno[Deno.internal].core;
    const db = (await import('db')).default;
    const jsqlx = (await import('jsqlx_core')).default;

    const conn = await db("_main");

    //load all dynamic modules
    var names = [];
    await conn.query(["SELECT name FROM nino_dynamic WHERE transpile = TRUE"], function (name) {
        names.push(name);
        return true;
    });

    for (var name of names) {
        core.print("transpiling: " + name + "...");
        //core.print("\n--------------------------- " + name + " ------------------------------------\n");
        var transpiled_code;
        // var encoder = new TextEncoder();
        await conn.query(["SELECT code FROM nino_dynamic WHERE name = $1", name], function (code) {
            // core.print("code typeof: " + (typeof code) + "\n");
            transpiled_code = jsqlx(code);
            //core.print(transpiled_code);
            //transpiled_code = encoder.encode(transpiled_code);
            return false;
        });

        await conn.query(["UPDATE nino_dynamic SET js = $2 WHERE name = $1", name, transpiled_code]);
        core.print("done\n");
    }
    //core.print("\n---------------------------------------------------------------\n");

    //core.print("names " + JSON.stringify(names) + "\n");

    // end with commit
    core.ops.op_tx_end(false);
    await core.opAsync('aop_end_task');
}

(async () => {
    await main();
})();