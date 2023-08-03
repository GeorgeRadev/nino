export default async function getDB(name) {
    debugger;

    const core = Deno[Deno.internal].core;
    const db_alias = await core.opAsync('aop_jsdb_get_connection_name', name);
    core.print('db alias :' + db_alias + '\n');

    var normalizeParams = function (args) {
        if (!Array.isArray(args)) {
            throw Error("query: first parameter must be an array of strings");
        }
        var params = [];
        var paramTypes = [];
        for (var arg of args) {
            params.push("" + arg);
            if (typeof arg == 'boolean') {
                paramTypes.push(0);
            } else if (typeof arg == 'number') {
                paramTypes.push(1);
            } else {
                paramTypes.push(2);
            }
        }
        return { params, paramTypes };
    }

    return {
        query: async function () {
            switch (arguments.length) {
                case 1: {
                    // [query array] ->  result set
                    var { params, paramTypes } = normalizeParams(arguments[0]);
                    return await core.opAsync('aop_jsdb_execute_query', name, params, paramTypes);
                }
                case 2: {
                    // [query array, callback]  
                    var { params, paramTypes } = normalizeParams(arguments[0]);
                    const rows = await core.opAsync('aop_jsdb_execute_query', name, params, paramTypes);
                    if (Array.isArray(rows)) {
                        const callback = arguments[1];
                        for (var row of rows) {
                            if (!callback.call(this, row)) {
                                break;
                            }
                        }
                    }
                    break;
                }

                default:
                    throw Error("query does not recognise those arguments");
            }
        },
        querySingle: async function () {
            switch (arguments.length) {
                case 1: {
                    // [query array] ->  result set
                    var { params, paramTypes } = normalizeParams(arguments[0]);
                    return await core.opAsync('aop_jsdb_execute_query_one', name, params, paramTypes);
                }
                case 2: {
                    // [query array, callback]  
                    var { params, paramTypes } = normalizeParams(arguments[0]);
                    const row = await core.opAsync('aop_jsdb_execute_query_one', name, params, paramTypes);
                    if (Array.isArray(row)) {
                        callback.call(this, row);
                    }
                    break;
                }

                default:
                    throw Error("query does not recognise those arguments");
            }
        }
    }
}