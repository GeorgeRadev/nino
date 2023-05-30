## Babel based transpiler from JSQLX (JSX+SQL) to js

```jsx
const n = 1;
const sql = SELECT id, username 
            FROM users 
            WHERE active = :active AND department=:('test'+n) AND department = 'test';
const html = <><h><span>{sql[0]}</span></h></>;
```

is beeing converted to:

```js
const n = 1;
const sql = [`SELECT id, username 
            FROM users 
            WHERE active =  $1  AND department= $2  AND department =  $3 `, active, 'test' + n, 'test'];
const html = _jsx(_Fragment, null, _jsx("h", null, _jsx("span", null, sql[0])));
```

## run
demo can be tested via:
```
node app.js
```
the production version is build via webpack and is in **./dist** folder
