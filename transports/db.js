export default async function getDB(name) {
    const core = Deno[Deno.internal].core;
    const db_alias = core.ops.op_tx_get_connection_name(name);
    core.print('db alias :' + db_alias + '\n');

    var normalizeParams = function (args) {
        if (!Array.isArray(args)) {
            throw Error("query: first parameter must be an array of strings");
        }
        var params = [];
        var paramTypes = [];
        for (var arg of args) {
            if (arg === undefined || arg === null) {
                params.push("NULL");
                paramTypes.push(0);
            } else if (typeof arg === 'boolean') {
                params.push("" + arg);
                paramTypes.push(1);
            } else if (typeof arg === 'number') {
                params.push("" + arg);
                paramTypes.push(2);
            } else if (arg instanceof Date) {
                params.push("" + arg.getUTCMilliseconds());
                paramTypes.push(4);
            } else {
                params.push("" + arg);
                paramTypes.push(3);
            }
        }
        return { params, paramTypes };
    }

    async function _query(queryArray, callback) {
        var { params, paramTypes } = normalizeParams(queryArray);

        if (params[0].startsWith("SELECT")) {
            const queryResult = core.ops.op_tx_execute_query(name, params, paramTypes);
            if (callback) {
                for (var row of queryResult.rows) {
                    if (!callback.call(this, row, queryResult.rowTypes, queryResult.rowNames)) {
                        break;
                    }
                }
                return undefined;
            } else {
                return queryResult;
            }
        } else {
            const queryResult = core.ops.op_tx_execute_upsert(name, params, paramTypes);
            return queryResult;
        }
    }

    return {
        // variants:
        // db.query(sql)
        // db.query(sql, callback(row, rowNames, rowTypes){})
        query: async function () {
            debugger;
            switch (arguments.length) {
                case 1: {
                    // [query array]
                    return await _query(arguments[0]);
                }
                case 2: {
                    // [query array, callback]  
                    if (typeof arguments[1] === "function") {
                        return await _query(arguments[0], arguments[1]);
                    } else {
                        throw new Error("Second parameters is expected to be number or callback function");
                    }
                }

                default:
                    throw Error("query does not recognise those arguments");
            }
        }
    }
}