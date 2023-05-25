'use strict';
import babel from '@babel/core';
import { sqlToArray } from "./jsql.js";

export default function jsqlx(code) {
    code = sqlToArray(code);
    const output = babel.transformSync(code, {
        plugins: [
            ["@babel/plugin-transform-react-jsx", {
                runtime: "classic",
                pragma: "_jsx",
                pragmaFrag: "_Fragment"
            }]
        ]
    });
    return output.code
}