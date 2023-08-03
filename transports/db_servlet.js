import db from 'db';

export default async function db_servlet(request) {
    debugger;

    var result = "<hr/>";
    {
        result += "<pre>";
        result += "// all with callback\n"
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
        // query one by one with callback
        result += "<pre>";
        result += "// all with callback and parameters\n"
        const conn = await db("_main");
        const sql = ["SELECT * FROM nino_database where db_alias = $1","_main"];

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
        // query single 
        result += "<pre>";
        result += "// query single line \n"
        const conn = await db("_main");
        const sql = ["SELECT * FROM nino_database where db_alias = $1","_main"];
        
        try{
            var row = await conn.querySingle(sql);
            result += "line single: " + JSON.stringify(row) + "\n";
        }catch(e){
            result += "Error: "+ JSON.stringify(e) ;
        }
        result += "</pre>";
        result += "<hr/>";
    }
    /*
    {
        // query with result_set
        const conn = await db("test");
        const sql = ["SELECT * FROM nino_setting"];

        result += "<pre>";
        result += "// query with resultSet \n"
        const resultSet = await conn.query(sql);
        result += "column names : " + JSON.stringify(resultSet.columns) + "\n";
        result += "column types : " + JSON.stringify(resultSet.columnTypes) + "\n";

        var line = 0;

        while (resultSet.next()) {
            result += "line " + (++line) + " : " + JSON.stringify(resultSet.row) + "\n";
        }
        result += "</pre>";
        result += "<hr/>";
    }
    */
    
    return result;
}