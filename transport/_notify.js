export default async function notify(message) {
    const core = Deno.core;

    if (typeof message === 'string' || message instanceof ArrayBuffer) {
        //ok
    } else if (typeof message === "number") {
        message = String.valueOf(message);
    } else {
        message = JSON.stringify(message);
    }
    return core.ops.nino_broadcast_message(message);
}