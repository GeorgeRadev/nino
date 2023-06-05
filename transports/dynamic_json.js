export default async function rest(request) {
    debugger;
    const core = Deno[Deno.internal].core;
    core.print('js_rest request:' + JSON.stringify(request) + '\n');
    let result = { method: request.method, path: request.path, time : new Date() };
    core.print('js_rest request:' + JSON.stringify(result) + '\n');
    return result;
}