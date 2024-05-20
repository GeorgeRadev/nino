/*
 * This is the main entry point for the nino js threads
 */
async function main() {
    const core = Deno.core;
    const module_invalidation_prefix = core.ops.nino_get_module_invalidation_prefix();
    const database_invalidation_prefix = core.ops.nino_get_database_invalidation_prefix();

    const header_set = function (key, value) {
        if (typeof key === 'string' && typeof value === 'string') {
            core.ops.nino_set_response_header(key, value);
        } else {
            throw new Error("response.set() parameters needs to be both strings not "
                + JSON.stringify(key) + ", "
                + JSON.stringify(value)
            );
        }
    };

    const response_status = function (status) {
        if (typeof status == 'number') {
            core.ops.nino_set_response_status(status);
        } else {
            throw new Error("response.status() needs to be a number not " + JSON.stringify(status));
        }
    };

    const get_body = function () {
        return core.ops.nino_get_request_body();
    };

    const send_response = async function (response) {
        // core.print('response typeof ' + (typeof response) + '\n');
        if (response === undefined || response === null) {
            throw new Error("response should not be undefined nor null");
        }
        if (typeof response === 'string') {
            await core.ops.nino_a_set_response_send_text(response);
        } else if (typeof response === "number") {
            await core.ops.nino_a_set_response_send_text(String.valueOf(response));
        } else if (response instanceof Uint8Array) {
            await core.ops.nino_a_set_response_send_buf(response);
        } else if (typeof response === 'object'
            && Object.hasOwn(response, 'proxy_fetch_result_as_response')
            && response.proxy_fetch_result_as_response == true) {
            var p = response;
            await core.ops.nino_a_set_response_from_fetch(p.url,
                p.timeout,
                p.method,
                p.headers,
                p.body);
        } else {
            await core.ops.nino_a_set_response_send_text(JSON.stringify(response));
        }
    };

    const get_jwt = function (username) {
        return core.ops.nino_get_user_jwt(username.toString());
    };

    // main loop
    for (; ;) {
        try {
            // core.print('_main try\n');
            const module = core.ops.nino_begin_task();

            debugger;
            if (module) {
                // request for module execution
                // core.print('module ' + module + '\n');
                const mod = await import(module);
                // core.print('after import ' + (typeof mod) + '\n');
                let handler = mod.default;
                if (!handler) {
                    throw new Error("module '" + module + "' has no export default async function");
                }
                if (typeof handler !== "function") {
                    throw new Error("module '" + module + "' export default async function is not a function");
                }

                const handler_arguments_count = handler.length;
                // core.print('default handler with ' + handler_arguments_count + ' arguments\n');
                const request = core.ops.nino_get_request();
                request.set = header_set;
                request.status = response_status;
                request.getBody = get_body;
                request.getJWT = get_jwt;

                if (handler_arguments_count <= 1) {
                    // rest handler with request param
                    // core.print('handler 1 request: ' + JSON.stringify(request) + '\n');
                    let response = await handler(request);
                    // core.print('result = ' + response + '\n');
                    await send_response(response);

                } else if (handler_arguments_count == 2) {
                    // servlet handler with request and response params
                    const response = {
                        set: header_set,
                        status: response_status,
                        send: async function (response) {
                            await send_response(response);
                        },
                    };

                    await handler(request, response);
                    // core.print('result = ' + (result) + '\n');

                } else {
                    throw new Error("module '" + module + "' default async function should take up to 2 parameters for rest and servlet modes");
                }

            } else {
                const invalidation_message = core.ops.nino_get_invalidation_message();
                if (invalidation_message) {
                    core.print('MSG:js: ' + invalidation_message + '\n');
                    //request for cache invalidation
                    if (invalidation_message.startsWith(module_invalidation_prefix)) {
                        // modules has been changed
                        // restart js engine to reset the compiled modules
                        break;
                    } else if (invalidation_message.startsWith(database_invalidation_prefix)) {
                        // reload databse aliases
                        core.ops.nino_reload_database_aliases();
                    } else {
                        // future js message listeners could be implemented here
                        // TODO: add dynamic dispatch to "exit_message_*"
                    }
                } else {
                    throw new Error("Should never get this");
                }
            }
            const commit = true;
            core.ops.nino_tx_end(commit);
            await core.ops.nino_a_broadcast_message(commit);
            await core.ops.nino_a_end_task();

        } catch (e) {
            try {
                let errorMessage = 'JS_ERROR: ' + e + '\n' + e.stack;
                core.print(errorMessage + '\n');
                core.ops.nino_set_response_status(500);
                core.ops.nino_set_response_header('Content-Type', 'text/plain;charset=UTF-8');
                await core.ops.nino_a_set_response_send_text(errorMessage);
            } catch (ex) {
                let errorMessage = 'JS_ERROR_ERR: ' + ex + '\n' + ex.stack;
                core.print(errorMessage + '\n');
            }
            try {
                const commit = false;
                core.ops.nino_tx_end(commit);
                await core.ops.nino_a_broadcast_message(commit);
                await core.ops.nino_a_end_task();
            } catch (ex) {
                let errorMessage = 'JS_ERROR_ERR: ' + ex + '\n' + ex.stack;
                core.print(errorMessage + '\n');
            }
        }
    }
}

(async () => {
    await main();
})();