export default async function notify(message) {
    debugger;
    const core = Deno[Deno.internal].core;
    if (typeof message === 'string' || message instanceof ArrayBuffer) {
        //ok
    } else if (typeof message === "number") {
        message = String.valueOf(message);
    } else {
        message = JSON.stringify(message);
    }
    return await core.opAsync('aop_broadcast_message', message);
}