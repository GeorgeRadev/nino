// this one is made as script because it is run as standalone jsqlx transpiling
async function main() {
    const core = Deno.core;
    const db = (await import('_db')).default;
    const jsqlx = (await import('_jsqlx')).default;

    try {
        const conn = await db();

        //load all dynamic modules
        var names = [];
        await conn.query(["SELECT response_name FROM nino_response WHERE transpile_flag = TRUE"], function (name) {
            names.push(name);
            return true;
        });

        for (var name of names) {
            core.print("transpiling: " + name + "...");

            var transpiled_code;
            await conn.query(["SELECT response_content FROM nino_response WHERE response_name = $1", name], function (code) {
                transpiled_code = jsqlx(code);
                return false;
            });

            await conn.query(["UPDATE nino_response SET javascript = $2 WHERE response_name = $1", name, transpiled_code]);
            core.print("done\n");
        }
        await conn.query(["UPDATE nino_response SET javascript_length = length(javascript) WHERE transpile_flag = TRUE"]);

        core.ops.nino_tx_end(true);
        await core.ops.nino_a_end_task();

    } catch (e) {
        let errorMessage = 'JS_ERROR: ' + e + '\n' + e.stack;
        core.print(errorMessage + '\n');
    }
}

(async () => {
    await main();
})();