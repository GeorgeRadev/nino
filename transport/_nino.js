import db from "_db";

export default class nino_core {

    static async isValidUserAndPassword(username, password) {
        if (!username || !password) {
            return false;
        }
        const core = Deno.core;
        const conn = await db();
        const sql = SELECT user_password 
                    FROM nino_user 
                    WHERE user_name = : username;
        var pass;
        await conn.query(sql, function (password) {
            pass = password;
            return false;
        });
        return core.ops.nino_password_verify(password, pass);
    }

}