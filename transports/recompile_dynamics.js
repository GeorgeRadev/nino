async function main() {
    const core = Deno[Deno.internal].core;
    
    const db = (await import('db')).default;
/*
    core.print('---before db main\n');
    const conn = await db("_main");
    core.print('---after db main\n');
    const sql = ["SELECT * FROM nino_database"];

    await conn.query(sql, function (row) {
        core.print("raw " + JSON.stringify(row) + "\n");
        return false;
    });
*/
}

(async () => {
    await main();
})();