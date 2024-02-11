export default async function fetch(url, options) {
    const core = Deno.core;
    debugger;

    if (!url || typeof url !== 'string') {
        throw new Error("first parameter must be a non empty string with url");
    }
    // options.timeout
    var timeout;
    if (options.timeout && typeof options.timeout == "number") {
        timeout = 0 | options.timeout;
    } else {
        timeout = 10000;
    }
    // options.method = [*GET, POST, PUT, DELETE, etc.]
    var method;
    if (options.method) {
        method = options.method.toString().toUpperCase();
    } else {
        method = "GET";
    }
    // options.headers = {key: value}
    const headers = new Map();
    if (options.headers && typeof options.headers === "object") {
        for (var [key, value] of Object.entries(options.headers)) {
            headers.set(key.toString(), value.toString());
        }
    }
    // options.body = string
    var body;
    if (options.body) {
        body = options.body.toString();
    } else {
        body = "";
    }
    const response = await core.ops.nino_a_fetch(url,
        timeout,
        method,
        headers,
        body);

    return {
        text: function () { return response },
        json: function () {
            return JSON.parse(response);
        }
    }
}