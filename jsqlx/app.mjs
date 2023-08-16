const jsqlx = await import("./src/jsqlx.mjs");

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
            WHERE db_alias = "nino_main";
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
var code2 = jsqlx.default(code);
var code3 = jsqlx.default(code2);
console.log("----------------");

if (code !== code2 || code2 !== code3 || code !== code3) {
    console.log("ERROR: transpining is not stable !!!");
} else {
    console.log(code);
    console.log("----------------");
}