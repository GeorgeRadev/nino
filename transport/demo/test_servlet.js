export default async function servlet(request, response) {
    await response.send('<hr/>method: ' + request.method + '<br/>path: '
        + request.path + '<hr/>'
        + JSON.stringify(request) + "<hr/>");
}