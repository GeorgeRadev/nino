import babel from '@babel/core';
import babel_jsx from '@babel/plugin-transform-react-jsx';
import { sqlToArray } from "./jsql.mjs";

export default function jsqlx(code) {
    code = sqlToArray(code);
    const output = babel.transformSync(code, {
        plugins: [
            [babel_jsx, {
                runtime: "classic",
                pragma: "_jsx",
                pragmaFrag: "_Fragment"
            }]
        ]
    });
    return output.code
}