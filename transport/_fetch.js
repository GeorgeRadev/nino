function validate_parameters(url, options) {
    // url
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

    return {
        url,
        timeout,
        method,
        headers,
        body
    }
}

/**
 * @param {*} url string
 * @param {*} options object with parameters
 * @returns text response and json
 */
export default async function fetch(url, options) {
    const core = Deno.core;

    var p = validate_parameters(url, options);
    const response = await core.ops.nino_a_fetch(p.url,
        p.timeout,
        p.method,
        p.headers,
        p.body);

    return {
        text: function () { return response },
        json: function () {
            return JSON.parse(response);
        }
    }
}

/**
 * @param {*} url string
 * @param {*} options object with parameters
 * @returns binary content of the response
 */
export async function fetch_binary(url, options) {
    const core = Deno.core;

    var p = validate_parameters(url, options);
    const response = await core.ops.nino_a_fetch_binary(p.url,
        p.timeout,
        p.method,
        p.headers,
        p.body);

    return response;
}

/**
 * @param {*} url string
 * @param {*} options object with parameters
 * @returns object that is recognised and request stream is send on the fly to the result
 */
export function fetch_response(url, options) {
    var p = validate_parameters(url, options);
    p.proxy_fetch_result_as_response = true;
    return p;
}