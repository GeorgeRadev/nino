import log from "_log";
import nino from "_nino";

export default async function servlet(request, response) {
    debugger;
    log('post body: ' + request.getBody() + '\n');
    log('post parameters: ' + JSON.stringify(request.postParameters) + '\n');

    response.status(302);

    if (request.postParameters && request.postParameters.username && request.postParameters.password) {
        const username = request.postParameters.username;
        const password = request.postParameters.password;
        var is_user_and_pass_ok = await nino.isValidUserAndPassword(username[0], password[0]);
        if (is_user_and_pass_ok) {
            response.set("Set-Cookie", "nino=" + request.getJWT("admin"));
            response.set("Location", "/portal");
            return "";
        }
    } else {
        response.set("Set-Cookie", "nino=");
        response.set("Location", "/login?error=user_and_pass_invalid");
        return "";
    }
}