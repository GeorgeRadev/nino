export default async function rest(request) {
    const core = Deno[Deno.internal].core;
    core.print('js_rest request:' + JSON.stringify(request) + '\n');
    let result = { method: request.method, path: request.path };
    core.print('js_rest request:' + JSON.stringify(result) + '\n');
    return result;
}