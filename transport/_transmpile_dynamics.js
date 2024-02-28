async function main() {
    const core = Deno.core;
    const db = (await import('_db')).default;
    const jsqlx = (await import('_jsqlx')).default;

    const conn = await db();

    //load all dynamic modules
    var names = [];
    await conn.query(["SELECT name FROM nino_dynamic WHERE transpile = TRUE"], function (name) {
        names.push(name);
        return true;
    });

    for (var name of names) {
        core.print("transpiling: " + name + "...");

        var transpiled_code;
        await conn.query(["SELECT code FROM nino_dynamic WHERE name = $1", name], function (code) {
            transpiled_code = jsqlx(code);
            return false;
        });

        await conn.query(["UPDATE nino_dynamic SET js = $2 WHERE name = $1", name, transpiled_code]);
        core.print("done\n");
    }

    // add admin user
    // {
    //     const username = "admin";
    //     const password = core.ops.nino_password_hash("admin");
    //     await conn.query(["UPDATE nino_user SET password = $2 WHERE username = $1", username, password]);
    // }
    // end with commit
    core.ops.nino_tx_end(true);
    await core.ops.nino_a_end_task();
}

(async () => {
    await main();
})();