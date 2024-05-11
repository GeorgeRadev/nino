import db from "_db";

export default class nino_core {

    /**
     * Verify user authentication
     * @param {string} username 
     * @param {string} password 
     * @returns true if chredentials are valid
     */
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

    /**
     * throws exception if parameters are invalid or 
     * the current authenticated user does not have the required role
     * @param {object} request 
     * @param {string} role 
     */
    static async assertRole(request, role) {
        if (!request || !request.user || !role) {
            throw new Error("no authenticated user");
        }
        const core = Deno.core;
        const conn = await db();
        const sql = SELECT user_role 
                    FROM nino_user_role 
                    WHERE user_name = : (request.user) 
                    AND user_role = : role;
        var hasRole = false;
        await conn.query(sql, function () {
            hasRole = true;
            return false;
        });
        if (!hasRole) {
            throw new Error("user does not have role: " + role);
        }
    }

    static async getPortletMenu(request) {
        if (!request || !request.user) {
            return [];
        }
        const core = Deno.core;
        const conn = await db();
        const sql = SELECT portlet_menu, portlet_icon, portlet_name
                    FROM nino_portlet p, nino_user_role ur
                    WHERE p.user_role = ur.user_role
                    AND ur.user_name = : (request.user)
                    ORDER BY p.portlet_index;

        var result = {};
        var separator_ix = 1;
        var menu_previous_tokens = [];
        await conn.query(sql, function (portlet_menu, portlet_icon, portlet_name) {
            var menu_tokens = portlet_menu.split("/");
            //check what is the difference with previous menu tokens
            for (var i = 0; i < menu_tokens.length - 1; i++) {
                if (menu_previous_tokens.length <= i || menu_previous_tokens[i] != menu_tokens[i]) {
                    // current token is new 
                    // add it to separators
                    result["separator" + separator_ix] = { name: menu_tokens[i], level: i };
                    separator_ix++;
                }
            }
            menu_previous_tokens = menu_tokens;

            result[portlet_menu] = {
                icon: portlet_icon,
                portlet: portlet_name,
                path: portlet_menu
            };
            return true;
        });
        return result;
    }

}