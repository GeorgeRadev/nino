import db from '_db';
import jsqlx from '_jsqlx';

export default async function db_servlet(request, response) {
    debugger;
    var result = "";
    const conn = await db();
    const sql = SELECT js 
                FROM nino_dynamic 
                WHERE name = 'portlet_test.js';
    await conn.query(sql, function (js) {
        result = js;
        return true;
    });

    if (request.query) {
        var transpiled_code = jsqlx(result);
        await conn.query(
            UPDATE nino_dynamic 
            SET js = : transpiled_code 
            WHERE name = 'portlet_test.js';
        );
    }

    await response.send(result);
}