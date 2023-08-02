export default async function getDB(name) {
    debugger;

    const core = Deno[Deno.internal].core;
    core.print('db alias :' + name + '\n');
    const db_alias = await core.opAsync('aop_jsdb_get_connection_name', name);
    return {
        query: async function () {
            switch (arguments.length) {
                case 1:
                    // [query array] ->  result set
                    if (!Array.isArray(arguments[0])) {
                        throw Error("first parameter must be an array of strings");
                    }
                    return {
                        columnNames: ["first", "second", "third"],
                        columnTypes: ["string", "int", "String"],
                        row: null,
                        current: 0,
                        max: 10,
                        next: function () {
                            if (this.current < this.max) {
                                this.current++;
                                this.row = ["aaa " + current, current, "bbbb"];
                                return true;
                            } else {
                                return false;
                            }
                        }
                    }

                case 2:
                    // [query array, callback]  
                    if (!Array.isArray(arguments[0])) {
                        throw Error("query: first parameter must be an array of strings");
                    }
                    if (typeof arguments[1] !== "function") {
                        throw Error("query: second parameter must be an array of strings");
                    }

                    core.print('db alias :' + name + '\n');
                    const rows = await core.opAsync('aop_jsdb_execute_query', name, arguments[0]);
                    if (Array.isArray(rows)) {
                        const callback = arguments[1];
                        for (var row of rows) {
                            core.print('db query row :' + JSON.stringify(row) + '\n');
                            if (!callback.call(this, row)) {
                                break;
                            }
                        }
                    }
                    break;

                default:
                    throw Error("query does not recognise those arguments");
            }
        },
        querySingle: async function () {

        },
        commit: async function () {

        },
        rollback: async function () {

        }
    }
}