export default async function getDB() {
    const core = Deno.core;

    var name;
    if (arguments.length > 0) {
        name = arguments[0].toString();
    } else {
        name = "_main";
    }
    const db_alias = core.ops.nino_tx_get_connection_name(name);
    core.print('db alias :' + db_alias + '\n');

    var normalizeParams = function (args) {
        if (!Array.isArray(args)) {
            throw Error("query: first parameter must be an array of strings");
        }
        var params = [];
        var paramTypes = [];
        for (var arg of args) {
            // core.print('p : ' + (typeof arg) + '\n');
            if (arg === undefined || arg === null) {
                params.push("NULL");
                paramTypes.push(0);
            } else if (arg instanceof Date) {
                params.push(arg.getUTCMilliseconds().toString());
                paramTypes.push(4);
            } else {
                if (typeof arg === 'boolean') {
                    paramTypes.push(1);
                } else if (typeof arg === 'number') {
                    paramTypes.push(2);
                } else {
                    paramTypes.push(3);
                }
                params.push(arg.toString());
            }
        }
        return { params, paramTypes };
    }

    async function _query(queryArray, callback) {
        var { params, paramTypes } = normalizeParams(queryArray);

        if (params[0].toUpperCase().startsWith("SELECT")) {
            const queryResult = core.ops.nino_tx_execute_query(name, params, paramTypes);
            if (callback) {
                for (var row of queryResult.rows) {
                    const params = [...row, queryResult.rowNames, queryResult.rowTypes];
                    if (!callback.apply(this, params)) {
                        break;
                    }
                }
                return undefined;
            } else {
                return queryResult;
            }
        } else {
            const queryResult = core.ops.nino_tx_execute_upsert(name, params, paramTypes);
            return queryResult;
        }
    }

    return {
        // variants:
        // db.query(sql)
        // db.query(sql, callback(row, rowNames, rowTypes){})
        query: async function () {
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
                    throw new Error("query does not recognise those arguments");
            }
        }
    }
}