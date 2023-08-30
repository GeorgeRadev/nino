import db from "_db";

export default class user {

    static async verifyUser(username, password) {
        if (!username || !password) {
            return false;
        }
        const core = Deno[Deno.internal].core;
        const conn = await db();
        const sql = SELECT password FROM nino_user WHERE username = : username;
        var pass;
        await conn.query(sql, function (password) {
            pass = password;
            return false;
        });
        return core.ops.op_password_verify(password, pass);
    }

}