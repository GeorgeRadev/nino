export default async function log() {
    const core = Deno[Deno.internal].core;
    if (arguments.length > 0) {
        for (var arg of arguments) {
            if (!arg) {
                core.print(new String(arg));
            }
            if (typeof arg === 'string') {
                core.print(arg);
            } else if (response instanceof ArrayBuffer) {
                core.print(arg);
            } else {
                core.print(JSON.stringify(arg));
            }
        }
    }
}