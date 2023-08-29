import log from "_log";
import user from "_user";

export default async function servlet(request, response) {
    debugger;
    log('post body: ' + request.getBody() + '\n');
    log('post parameters: ' + JSON.stringify(request.postParameters) + '\n');

    response.status(302);

    if (request.postParameters && request.postParameters.username && request.postParameters.password) {
        const username = request.postParameters.username;
        const password = request.postParameters.password;
        var is_user_and_pass_ok = await user.verifyUser(username[0], password[0]);
        if (is_user_and_pass_ok) {
            response.set("Set-Cookie", "nino=" + request.getJWT("admin"));
            response.set("Location", "/test_servlet");
            return "";
        }
    }
    response.set("Set-Cookie", "nino=" + request.getJWT("admin"));
    response.set("Location", "/login?error-user_and_pass_invalid");
    return "";
}