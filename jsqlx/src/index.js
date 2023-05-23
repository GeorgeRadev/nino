import babel from '@babel/core';
import { sqlToArray } from "./jsql.js"

var code = `
const n = 1;
const sql = SELECT id, username 
            FROM users 
            WHERE active = :active AND department=:('test'+n) AND department = 'test';
const html = <><h><span>{sql[0]}</span></h></>;
`;

console.log("----------------");
console.log(code);

// try convert sql to array
console.log("---sql-------------");
code = sqlToArray(code);
console.log(code);

const output = babel.transformSync(code, {
    plugins: [
        ["@babel/plugin-transform-react-jsx", {
            runtime: "classic",
            pragma: "_jsx",
            pragmaFrag: "_Fragment"
        }]
    ]
});

console.log("----------------");
console.log(output.code);
console.log("----------------");
