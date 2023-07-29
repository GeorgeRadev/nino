import path from "path";

const dist = path.resolve("./dist");

export default {
    mode: "production",
    entry: {
        jsqlx: './src/jsqlx.mjs',
    },
    output: {
        path: dist,
        filename: "[name].min.js",
        library: {
            type: "module",
        }
    },
    experiments: {
        outputModule: true,
    },
};
