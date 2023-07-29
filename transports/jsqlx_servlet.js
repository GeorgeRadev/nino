import jsqlx from 'jsqlx_core';

export default async function jsqlx_servlet(request, response) {
    debugger;
    
    var code = `
    const n = 1;
    const sql = SELECT id, username 
    FROM users 
    WHERE active = :active AND department=:('test'+n) AND department = 'test';
    const html = <><h><span>{sql[0]}</span></h></>;
    `;
    
    var code_trans = jsqlx(code);
    
    var result = "<hr/>";
    result+="<pre>";
    result+=code;
    result+="</pre>";
    result+="<hr/>";
    result+="<pre>";
    result+=code_trans;
    result+="</pre>";
    result+="<hr/>";
    
    response.set('Content-Type', 'text/html;charset=UTF-8');
    await response.send(result);
}