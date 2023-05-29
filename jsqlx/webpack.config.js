const path = require('path')

const dist = path.resolve("./dist");

module.exports = {
    mode: "production",
    entry: {
        //app: './src/app.mjs',
        jsqlx: './src/jsqlx.mjs',
    },
    output: {
        path: dist,
        filename: "[name].min.js"
    },
    target: "node",
}

