export default async function log() {
    const core = Deno.core;

    if (arguments.length > 0) {
        let log_message = "";
        for (var arg of arguments) {
            if (typeof arg === 'string') {
                log_message += arg;
            } else {
                log_message += JSON.stringify(arg);
            }
        }

        core.print(log_message);
        await core.ops.nino_a_log(log_message);
    }
}