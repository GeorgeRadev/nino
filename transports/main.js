async function main() {
    for (; ;) {
        const core = Deno[Deno.internal].core;
        try {
            // core.print('js try\n');
            debugger;
            const module = core.ops.op_begin_task();

            if (module) {
                // request for module execution
                // core.print('module ' + module + '\n');
                const mod = await import(module);
                // core.print('after import ' + (typeof mod) + '\n');
                let handler = mod.default;
                if (!handler) {
                    throw Exception("module '" + module + "' has no export default async function");
                }
                if (typeof handler !== "function") {
                    throw Exception("module '" + module + "' export default async function is not a function");
                }

                const handler_arguments_count = handler.length;
                // core.print('default handler with ' + handler_arguments_count + ' arguments\n');

                const request = core.ops.op_get_request();
                request.set = function (key, value) {
                    core.ops.op_set_response_header(key, value);
                };
                const send_response = async function (response) {
                    core.print('response typeof ' + (typeof response) + '\n');
                    if (typeof response === 'string') {
                        await core.opAsync('aop_set_response_send_text', response);
                    } else if (response instanceof Number) {
                        await core.opAsync('aop_set_response_send_text', String.valueOf(response));
                    } else if (response instanceof ArrayBuffer) {
                        await core.opAsync('aop_set_response_send_buf', response);
                    } else {
                        await core.opAsync('aop_set_response_send_json', JSON.stringify(response));
                    }
                }

                if (handler_arguments_count == 1) {
                    // rest handler with request param
                    // core.print('handler 1 request: ' + JSON.stringify(request) + '\n');
                    let response = await handler(request);
                    // core.print('after handler response' + response + '\n');
                    await send_response(response);

                } else if (handler_arguments_count == 2) {
                    // servlet handler with request and response params
                    const response = {
                        set: function (key, value) {
                            if (key instanceof String && value instanceof String) {
                                core.ops.op_set_response_header(key, value);
                            } else {
                                throw Exception("response.set() parameters needs to be both strings not "
                                    + JSON.stringify(key) + ", "
                                    + JSON.stringify(value)
                                );
                            }
                        },
                        status: function (status) {
                            if (status instanceof Number) {
                                core.ops.op_set_response_status(status);
                            } else {
                                throw Exception("response.status() needs to be a number not " + JSON.stringify(status));
                            }
                        },
                        send: async function (response) {
                            await send_response(response);
                        },
                    };

                    core.print('before handler 2\n');
                    core.ops.op_set_response_header('Content-Type', 'text/html;charset=UTF-8');
                    await handler(request, response);
                    // core.print('result = ' + (result) + '\n');
                    core.print('after handler 2\n');

                } else {
                    throw Exception("module '" + module + "' default async function should take 1 or 2 parameters for rest and servlet modes");
                }

            } else {
                const invalidation_message = core.ops.op_get_invalidation_message();
                if (invalidation_message) {
                    //request for cache invalidation
                    if (invalidation_message.startsWith(core.ops.op_get_module_invalidation_prefix())) {
                        // modules has been changed
                        const threadId = core.ops.op_get_thread_id();
                        core.print('js got invalidation message (' + threadId + '): ' + invalidation_message + '\n');
                        break;
                    }
                } else {
                    throw Exception("Should never get this");
                }
            }
            await core.opAsync('aop_end_task', false);

        } catch (e) {
            try {
                let errorMessage = 'JS_ERROR: ' + e + '\n' + e.stack;
                core.print(errorMessage + '\n');
                core.ops.op_set_response_status(500);
                core.ops.op_set_response_header('Content-Type', 'text/plain;charset=UTF-8');
                await core.opAsync('aop_set_response_send_text', errorMessage);
                await core.opAsync('aop_end_task', true);
            } catch (ex) {
                let errorMessage = 'JS_ERROR_ERR: ' + e + '\n' + e.stack;
                core.print(errorMessage + '\n');
            }
        }
    }
}

(async () => {
    await main();
})();