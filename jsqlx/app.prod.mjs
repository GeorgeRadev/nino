const jsqlx = await import("./dist/jsqlx.min.js");

var code = `
{
const n = 1;
const sql = SELECT id, username 
            FROM users 
            WHERE active = :active AND department=:('test'+n) AND department = 'test';
}
{
const sql = SELECT * 
            FROM nino_database 
            WHERE db_alias = "_main";
await conn.query(sql, function (row) {
    result += "line " + (++line) + " : " + JSON.stringify(row) + "\\n";
    // return true to fetch next
    return false;
});
}
const html = <><h><span>{sql[0]}</span></h></>;
`;

console.log("----------------");
console.log(code);

code = jsqlx.default(code);
console.log("----------------");
console.log(code);
console.log("----------------");