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

    static async ninoRequestsGet() {
        const conn = await db();
        const sql = SELECT request_path, response_name, redirect_flag, authorize_flag 
                    FROM nino_request 
                    ORDER BY request_path;
        var result = [];
        await conn.query(sql, function (request_path, response_name, redirect_flag, authorize_flag) {
            result.push({
                request_path: request_path,
                redirect_flag: redirect_flag,
                authorize_flag: authorize_flag,
                response_name: response_name,
            });
            return true;
        });
        return result;
    }

    static async ninoRequestsDetail(name) {
        const conn = await db();
        const sql = SELECT request_path, response_name, redirect_flag, authorize_flag
                    FROM nino_request 
                    WHERE request_path = : name;

        var result;
        await conn.query(sql, function (request_path, response_name, redirect_flag, authorize_flag) {
            result = {
                request_path: request_path,
                response_name: response_name,
                redirect_flag: redirect_flag,
                authorize_flag: authorize_flag,
            };
            return false;
        });
        return result;
    }

    static async ninoResponsesGet() {
        const conn = await db();
        const sql = SELECT response_name, response_mime_type, execute_flag, transpile_flag
                    FROM nino_response 
                    ORDER BY response_name;

        var result = [];
        await conn.query(sql, function (response_name, response_mime_type, execute_flag, transpile_flag) {
            result.push({
                response_name: response_name,
                response_mime_type: response_mime_type,
                execute_flag: execute_flag,
                transpile_flag: transpile_flag
            });
            return true;
        });
        return result;
    }

    static async ninoResponsesDetail(name) {
        const conn = await db();
        const sql = SELECT response_name, response_mime_type, execute_flag, transpile_flag, response_content, javascript
                    FROM nino_response 
                    WHERE response_name = : name;

        var result;
        await conn.query(sql, function (response_name, response_mime_type, execute_flag, transpile_flag, response_content, javascript) {
            result = {
                response_name: response_name,
                response_mime_type: response_mime_type,
                execute_flag: execute_flag,
                transpile_flag: transpile_flag,
                response_content, response_content,
                javascript: javascript
            };
            return false;
        });
        return result;
    }

    static async ninoUsersRolesGet() {
        const conn = await db();
        const sql = SELECT user_name, user_role
                    FROM nino_user_role 
                    ORDER BY user_name, user_role;

        var result = [];
        await conn.query(sql, function (user_name, user_role) {
            result.push({
                user_name: user_name,
                user_role: user_role
            });
            return true;
        });
        return result;
    }

    static async ninoPortletsGet() {
        const conn = await db();
        const sql = SELECT user_name, user_role
                    FROM nino_user_role 
                    ORDER BY user_name, user_role;

        var result = [];
        await conn.query(sql, function (user_name, user_role) {
            result.push({
                user_name: user_name,
                user_role: user_role
            });
            return true;
        });
        return result;
    }

    static async ninoPortletsGet() {
        const conn = await db();
        const sql = SELECT user_role, portlet_menu, portlet_index, portlet_icon, portlet_name
                    FROM nino_portlet 
                    ORDER BY user_role, portlet_menu;

        var result = [];
        await conn.query(sql, function (user_role, portlet_menu, portlet_index, portlet_icon, portlet_name) {
            result.push({
                user_role: user_role,
                portlet_menu: portlet_menu,
                portlet_index: portlet_index,
                portlet_icon: portlet_icon,
                portlet_name: portlet_name
            });
            return true;
        });
        return result;
    }

    static async ninoSettingsGet() {
        const conn = await db();
        const sql = SELECT setting_key, setting_value
                    FROM nino_setting 
                    ORDER BY setting_key;

        var result = [];
        await conn.query(sql, function (setting_key, setting_value) {
            result.push({
                setting_key: setting_key,
                setting_value: setting_value
            });
            return true;
        });
        return result;
    }

    static async ninoDatabasesGet() {
        const conn = await db();
        const sql = SELECT db_alias, db_type, db_connection_string
                    FROM nino_database 
                    ORDER BY db_alias;

        var result = [];
        await conn.query(sql, function (db_alias, db_type, db_connection_string) {
            result.push({
                db_alias: db_alias,
                db_type: db_type,
                db_connection_string: db_connection_string
            });
            return true;
        });
        return result;
    }

    static async ninoDatabaseQuery(alias, query) {
        try {
            const conn = await db(alias);
            var cols = [];
            var rows = [];

            await conn.query([query], function () {
                const len = arguments.length;
                if (len < 3) {
                    throw new Error("not enough result to display in table");
                }
                if (cols.length <= 0) {
                    for (var i = 0; i < arguments[len - 1].length; i++) {
                        cols.push({ name: arguments[len - 2][i], type: arguments[len - 1][i] });
                    }
                }
                var row = [];
                for (var i = 0; i < len - 2; i++) {
                    row.push(arguments[i]);
                }
                rows.push(row);
                return true;
            });
            return { cols: cols, rows: rows, error: "" };

        } catch (e) {
            let errorMessage = '' + e;
            return { error: errorMessage };
        }
    }

    static async ninoLogsGet(limit) {
        const conn = await db();
        const sql = SELECT to_char(log_timestamp, 'YYYY-MM-DD HH24:MI:SS'), method, request, response, log_message
                    FROM nino_log 
                    ORDER BY log_timestamp DESC
                    LIMIT :limit;

        var result = [];
        await conn.query(sql, function (log_timestamp, method, request, response, message) {
            result.push({
                log_timestamp: log_timestamp,
                method: method,
                request: request,
                response: response,
                message: message
            });
            return true;
        });
        return result;
    }
}