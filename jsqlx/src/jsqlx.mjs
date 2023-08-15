import babel from '@babel/standalone';
import { sqlToArray } from "./jsql.mjs";

export default function jsqlx(code) {
    code = sqlToArray(code);
    const output = babel.transform(code, {
        presets: [
            ['react', {
                runtime: "automatic",
                // pragma: "_jsx",
                // pragmaFrag: "_Fragment"
            }]]
    });
    return output.code.toString();
}