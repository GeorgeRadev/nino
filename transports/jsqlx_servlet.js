import jsqlx from '_jsqlx';

export default async function jsqlx_servlet(request, response) {
    debugger;
    
    var code = `
    var line = 0;
    const sql = SELECT * 
                FROM nino_database;
    await conn.query(sql, function (row) {
        result += "line " + (++line) + " : " + JSON.stringify(row) + "\\n";
        // return true to fetch next
        return true;
    });

    const html = <><h><span>{sql[0]}</span></h></>;
    `;
    
    var code_trans = jsqlx(code);
    
    var result = "<hr/>";
    result+="<pre>";
    result+=code;
    result+="</pre>";
    result+="<hr/>";
    result+="<pre>";
    result+=code_trans;
    result+="</pre>";
    result+="<hr/>";
    
    response.set('Content-Type', 'text/html;charset=UTF-8');
    await response.send(result);
}