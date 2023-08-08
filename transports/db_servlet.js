import db from 'db';

export default async function db_servlet(request) {
    debugger;

    var result = "<hr/>";
    {
        result += "<pre>";
        result += "// all with callback  \n"
        const conn = await db("_main");
        const sql = ["SELECT * FROM nino_database"];

        var line = 0;
        await conn.query(sql, function (row) {
            result += "line " + (++line) + " : " + JSON.stringify(row) + "\n";
            // return true to fetch next
            return true;
        });

        result += "</pre>";
        result += "<hr/>";
    }

    {
        result += "<pre>";
        result += "// one with callback and parameters\n"
        const conn = await db("_main");

        const sql = ["SELECT * FROM nino_database where db_alias = $1", "_main"];
        await conn.query(sql, function (row) {
            result += "line " + (++line) + " : " + JSON.stringify(row) + "\n";
            // return true to fetch next
            return false;
        });

        // update _main description to be the current Date
        var currentDate = (new Date()).toString();
        if (request.query) {
            currentDate = "error";
        }
        const update = ["UPDATE nino_database SET db_connection_string = $2 where db_alias = $1", "_main", currentDate];
        var affected = await conn.query(update);
        result += "affected " + affected + " lines \n";

        // show update
        await conn.query(sql, function (row) {
            result += "line " + (++line) + " : " + JSON.stringify(row) + "\n";
            // return true to fetch next
            return false;
        });

        if (request.query) {
            throw new Error("test rollback");
        }

        result += "request : " + JSON.stringify(request) + "\n";
        result += "</pre>";
        result += "<hr/>";
    }
    {
        result += "<pre>";
        result += "// query result \n"
        const conn = await db("_main");
        const sql = ["SELECT * FROM nino_database where db_alias = $1", "_main"];

        try {
            var queryResult = await conn.query(sql);
            result += "result: " + JSON.stringify(queryResult) + "\n";
        } catch (e) {
            result += "Error: " + JSON.stringify(e);
        }
        result += "</pre>";
        result += "<hr/>";
    }

    return result;
}