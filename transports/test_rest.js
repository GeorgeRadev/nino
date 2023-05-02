export default async function rest(request) {
    Deno.core.print('js_rest request:' + request + '\n');
    let result = { method: request.method, path: request.path };
    Deno.core.print('js_rest request:' + result + '\n');
    return result;
}