import db from 'db';

export default async function db_servlet(request) {
    debugger;

    var result = "<hr/>";
    {
        // query one by one with callback
        const conn = db("_main");
        const sql = ["SELECT * FROM nino_database"];

        result += "<pre>";
        result += "// all with callback\n"
        var line = 0;
        conn.query(sql, function (row) {
            result += "line " + (++line) + " : " + JSON.stringify(row) + "\n";
            // return true to fetch next
            return true;
        });
        result += "</pre>";
        result += "<hr/>";
    }
    {
        // query single 
        const conn = db("_main");
        const sql = ["SELECT * FROM nino_database"];

        result += "<pre>";
        result += "// query single line \n"
        var row = conn.querySingle(sql);
        var line = 0;
        if (row) {
            result += "line " + (++line) + " : " + JSON.stringify(row) + "\n";
        }
        result += "</pre>";
        result += "<hr/>";
    }
    {
        // query with result_set
        const conn = db("test");
        const sql = ["SELECT * FROM nino_setting"];

        result += "<pre>";
        result += "// query with resultSet \n"
        const resultSet = conn.query(sql);
        result += "column names : " + JSON.stringify(resultSet.columns) + "\n";
        result += "column types : " + JSON.stringify(resultSet.columnTypes) + "\n";

        var line = 0;

        while (resultSet.next()) {
            result += "line " + (++line) + " : " + JSON.stringify(resultSet.row) + "\n";
        }
        result += "</pre>";
        result += "<hr/>";
    }

    request.set('Content-Type', 'text/html;charset=UTF-8');
    await result;
}