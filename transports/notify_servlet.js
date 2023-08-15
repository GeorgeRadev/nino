import notify from 'notify';

export default async function servlet(request, response) {
    debugger;

    const message1 = "dynamic:" + new Date();
    await notify(message1);

    const message2 = "database:" + new Date();
    await notify(message2);

    var res = response.set('Content-Type', 'text/html;charset=UTF-8');
    await response.send('<hr/>' + message1 + "<br/>" + message2 + '<hr/>');
    return res;
}