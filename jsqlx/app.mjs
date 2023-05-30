const jsqlx = await import("./src/jsqlx.mjs");

var code = `
const n = 1;
const sql = SELECT id, username 
            FROM users 
            WHERE active = :active AND department=:('test'+n) AND department = 'test';
const html = <><h><span>{sql[0]}</span></h></>;
`;

console.log("----------------");
console.log(code);

code = jsqlx.default(code);
console.log("----------------");
console.log(code);
console.log("----------------");