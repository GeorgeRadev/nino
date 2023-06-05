export default async function servlet(request, response) {
    debugger;
    const core = Deno[Deno.internal].core;
    core.print('js_servlet request: ' + JSON.stringify(request) + '\n');
    response.set('Content-Type', 'text/html;charset=UTF-8');
    await response.send('<hr/>method: ' + request.method + '<br/>path: ' + request.path + '</hr/>');
}