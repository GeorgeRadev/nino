export default async function servlet(request) {
    debugger;
    var body = request.getBody();

    request.set("Set-Cookie", "nino=" + request.getJWT("admin"));

    var result = "";
    result += '<hr/>method: ' + request.method + '<br/>';
    result += 'path: ' + request.path + '<hr/>';
    result += JSON.stringify(request) + '<hr/>';
    result += 'body: <br/>' + body + '<hr/>';

    return result;
}