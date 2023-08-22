export default async function servlet(request) {
    debugger;
    var body = request.getBody();

    var result = "";
    result += '<hr/>method: ' + request.method + '<br/>';
    result += 'path: ' + request.path + '<hr/>';
    result += JSON.stringify(request) + '<hr/>';
    result += 'body: <br/>' + body + '<hr/>';

    return result;
}