export default async function rest(request) {
    debugger;
    let result = { method: request.method, path: request.path, time: new Date() };
    return result;
}