import db from '_db';
import jsqlx from '_jsqlx';

export default async function db_servlet(request, response) {
    debugger;
    var result = "";
    const conn = await db();
    const sql = SELECT javascript 
                FROM nino_dynamic 
                WHERE dynamic_name = 'portlet_counter.js';
    await conn.query(sql, function (js) {
        result = js;
        return true;
    });

    if (request.query) {
        var transpiled_code = jsqlx(result);
        await conn.query(
            UPDATE nino_dynamic 
            SET javascript = : transpiled_code 
            WHERE dynamic_name = 'portlet_counter.js';
        );
    }

    await response.send(result);
}