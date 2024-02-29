# NINO

Scalable distributed Javascript platform for developing web portals/services.  
Uses deno v8 for executing Javascript in isolated distributed environment.  
Ultimate goal is to be an accelerator for web-based solutions.

# JSQLX
The JS environment can transpile a jsx dialiect called **JSQLX**:  
Javascript with jsx(react) transpiling and SQL to array convertion.  
This elevates the js to be a [Fourth-generation programming language](https://en.wikipedia.org/wiki/Fourth-generation_programming_language),  
and removes the sql injection by moving strings and variables as  
prepared statement parameters to be escaped properly.

The conversion/transpiling can be explained better with the following example:

```jsx
var line = 0;
const sql =
    SELECT db_alias, db_type, db_connection_string
    FROM nino_database 
    WHERE db_alias = "_main";

await conn.query(sql, function () {
    result += "line " + (++line) + " : " + JSON.stringify(arguments) + "\n";
    // return true to fetch next
    return false;
});

const html = <><h><span>{sql[0]}</span></h></>;
```

will be transpiled as:
```js 
import { jsx as _jsx } from "react/jsx-runtime";
import { Fragment as _Fragment } from "react/jsx-runtime";

var line = 0;
const sql = [`SELECT db_alias, db_type, db_connection_string
              FROM nino_database 
              WHERE db_alias =  $1 `, "_main"];

await conn.query(sql, function () {
  result += "line " + ++line + " : " + JSON.stringify(arguments) + "\n";
  // return true to fetch next
  return false;
});

const html = /*#__PURE__*/_jsx(_Fragment, {
  children: /*#__PURE__*/_jsx("h", {
    children: /*#__PURE__*/_jsx("span", {
      children: sql[0]
    })
  })
});
```

Using the same transpiler for frontend (jsx) and backend(jsql) code.  
The transpiler source is in **/jsqlx** folder and it is based on the **babel** static jsx transpiler.


## Setting up the test environment

you need Linux or MacOS environment for this:

- Build the application: **cargo build**  
- Start the postgres db:  **./db start**  
- Export NINO environment variable: **export NINO=postgresql://your_user_name@localhost/postgres?connect_timeout=5**  
- Build initial db script: **./build _transport_sql**  
- Start the application: **cargo run**  
- Open **http://localhost:8080/** to play with the environment

to stop the application just terminate it (ctrl-c usually do the trick).  
to stop the db : **./db stop**  


## Dependencies
Requires postgreSQL for storing all data, code, configuration and message broadcasting.  
the test environment depend on the **zonky** postgres binary builds.


## Loading sequence:

- check database for existance and connect to DB
- create mem cache for settings. 
- create DB message listener/broadcaster.
- create local(./cache/...) cache for static resources (*not implemented yet*)
- create local dynamic javascript threads based on the settings
- create local web server, and dispach requests to the static/dynamic content.


## Components

DBManager - used only internaly for extracting database info.
DBNotificationManager - gives connections and serves also as messenger for broadcasting and receiving messages
  - getConnectionFromPool
  - addMessageListener
  - getResourcesByPath
SettingsMananager - contains all settings cached for fast access and invalidates them on notification
DispatchManager - gets web requests and decides to use the Static or Dynamic Manager
StaticManager - keeps localy static resources as files in ./cache and invalidates them on notification
              - static resources can be marked as public for they will be visible for serving
DynamicManager - keeps track on dynamic modules localy in ./js  and invalidates them on notification
               - dynamic resources can be REST or JS for calling default(requestObject) -> ResponceObject
               - or default(request, response) -> void for servlet-like implementations
               - will have multiple JavaScript Instances with separate execution context and process tasks
WebManager - listnens on port and upon requests decides what dynqmic or static resource to serve.
               - the static ones needs to match exact path, while dynamic will be resolved upon path mathing
               - keeps track on JWT token in the header for authentication
               - JWT tokens are verified if Dynamic resource is marked as secured otherwize just passed

requirements:
to have a dynamic module "_main" for executing the dynamic executions.
