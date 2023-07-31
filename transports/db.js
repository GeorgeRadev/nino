
const core = Deno[Deno.internal].core;

async function getDB(name) {

    const db_alias = await core.opAsync('aop_db_connect', name);
    return {
        query: function () {

        },
        querySingle: function () {

        },
        commit: function () {

        },
        rollback: function () {

        }
    }
}

export default getDB;