import db from "_db";

export default class user {

    static async verifyUser(username, password) {
        if (!username || !password) {
            return false;
        }
        const core = Deno.core;
        const conn = await db();
        const sql = SELECT password FROM nino_user WHERE username = : username;
        var pass;
        await conn.query(sql, function (password) {
            pass = password;
            return false;
        });
        return core.ops.nino_password_verify(password, pass);
    }

}