import log from "_log";

export default async function servlet(request, response) {
    debugger;
    log('post body: ' + request.getBody() + '\n');
    log('post parameters: ' + JSON.stringify(request.postParameters) + '\n');

    response.set("Set-Cookie", "nino=" + request.getJWT("admin"));
    // redirect to GET test_servlet 
    response.set("Location", "/test_servlet");
    response.status(302);
    return "";
}