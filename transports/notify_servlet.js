import notify from 'notifier';

export default async function servlet(request, response) {
    debugger;
    const message = "broadcast date : " + new Date();
    await notify(message);
    var res = response.set('Content-Type', 'text/html;charset=UTF-8');
    await response.send('<hr/>' + message + '<hr/>');
    return res;
}