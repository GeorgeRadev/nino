// activate await functionality
Deno.core.initializeAsyncOps();

async function main() {
    for (; ;) {
        try {
            Deno.core.print('try\n');
            const module = Deno.core.ops.op_begin_task();
            if (module) {
                Deno.core.print('module ' + module + '\n');
                const mod = await import(module);
                Deno.core.print('after import ' + (typeof mod) + '\n');
                let handler = mod.default;
                if (!handler) {
                    throw Exception("module '" + module + "' has no export default async function");
                }
                if (typeof handler !== "function") {
                    throw Exception("module '" + module + "' export default async function is not a function");
                }

                const handler_arguments_count = handler.length;
                Deno.core.print('default handler with ' + handler_arguments_count + ' arguments\n');

                const request = Deno.core.ops.op_get_request();
                const send_response = async function (response) {
                    Deno.core.print('response typeof ' + (typeof response) + '\n');
                    if (typeof response === 'string') {
                        await Deno.core.ops.aop_set_response_send_text(response);
                    } else if (response instanceof Number) {
                        await Deno.core.ops.aop_set_response_send_text(String.valueOf(response));
                    } else if (response instanceof ArrayBuffer) {
                        await Deno.core.ops.aop_set_response_send_buf(response);
                    } else {
                        await Deno.core.ops.aop_set_response_send_json(JSON.stringify(response));
                    }
                }

                if (handler_arguments_count == 1) {
                    // rest handler with request param
                    Deno.core.print('handler 1 request: ' + JSON.stringify(request) + '\n');
                    let response = await handler(request);
                    Deno.core.print('after handler response' + response + '\n');
                    await send_response(response);

                } else if (handler_arguments_count == 2) {
                    // servlet handler with request and response params
                    const response = {
                        set: function (key, value) {
                            Deno.core.ops.op_set_response_header(key, value);
                        },
                        status: function (status) {
                            Deno.core.ops.op_set_response_status(status);
                        },
                        send: async function (response) {
                            await send_response(response);
                        },
                    };

                    Deno.core.print('before handler 2\n');
                    let result = await handler(request, response);
                    Deno.core.print('result = ' + (result) + '\n');
                    Deno.core.print('after handler 2\n');

                } else {
                    throw Exception("module '" + module + "' default async function should take 1 or 2 parameters for rest and servlet modes");
                }
            }
        } catch (e) {
            let errorMessage = 'JS_ERROR: ' + e + '\n' + e.stack;
            Deno.core.print(errorMessage + '\n');
            Deno.core.ops.op_set_response_status(500);
            await Deno.core.ops.aop_set_response_send_text(errorMessage);
        } finally {
            await Deno.core.ops.aop_end_task();
        }
    }
}

(async () => {
    await main();
})();