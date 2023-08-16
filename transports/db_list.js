import db from 'db';
import jsqlx from 'jsqlx_core';

export default async function db_servlet(request, response) {
    debugger;
    request.set('Content-Type', 'text/plain;charset=UTF-8');
    var result = "";
    const conn = await db("nino_main");
    const sql = ['SELECT js FROM nino_dynamic WHERE name = $1', 'db_servlet'];
    await conn.query(sql, function (js) {
        result = js;
        return true;
    });

    if (request.query) {
        var transpiled_code = jsqlx(result);
        await conn.query(["UPDATE nino_dynamic SET js = $2 WHERE name = $1", 'db_servlet', transpiled_code]);
    }

    await response.send(result);
}