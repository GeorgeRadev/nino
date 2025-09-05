import db from '_db';

export default async function db_servlet(request) {
    var result = "<hr/>";
    {
        result += "<pre>";
        result += "// all with callback  \n"
        const conn = await db();

        var line = 0;
        const sql =
            SELECT *
            FROM nino_database;
        await conn.query(sql, function (db_alias, db_type, db_connection_string) {
            result += "line " + (++line) + " : " + db_alias + ", " + db_type + ", " + db_connection_string + "\n";
            // return true to fetch next
            return true;
        });

        result += "</pre>";
        result += "<hr/>";
    }
    {
        result += "<pre>";
        result += "// one with callback and parameters\n"
        const conn = await db();

        var line = 0;
        const sql =
            SELECT db_alias, db_type, db_connection_string
            FROM nino_database 
            WHERE db_alias = "_main";
        await conn.query(sql, function () {
            result += "line " + (++line) + " : " + JSON.stringify(arguments) + "\n";
            // return true to fetch next
            return false;
        });

        // update _main description to be the current Date
        var currentDate = (new Date()).toString();
        if (request.query) {
            currentDate = "error";
        }
        const update =
            UPDATE nino_database 
            SET db_connection_string = :currentDate 
            WHERE db_alias = "_main";
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
        const conn = await db("test");
        const sql =
            SELECT *
            FROM nino_database 
            WHERE db_alias = "test";
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