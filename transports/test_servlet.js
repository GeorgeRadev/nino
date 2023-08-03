export default async function servlet(request, response) {
    debugger;
    const core = Deno[Deno.internal].core;
    core.print('js_servlet request: ' + JSON.stringify(request) + '\n');
    await response.send('<hr/>method: ' + request.method + '<br/>path: '
        + request.path + '<hr/>'
        + JSON.stringify(request) + "<hr/>");
}