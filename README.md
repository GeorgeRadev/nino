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

In Linux or MacOS environment execute following steps:

- Build the application: **cargo build**  
- Start the postgres db:  **./db start**  
- Export NINO environment variable: **export NINO=postgresql://your_user_name@localhost/postgres?connect_timeout=5**  
- Build initial db script: **./build _transport_sql**  
- Start the application: **cargo run**  
- Open **http://localhost:8080/** to play with the environment

to stop the application just terminate it (ctrl-c usually do the trick).  
to stop the postgres db : **./db stop**  


## Dependencies
Requires postgreSQL for storing all data, code, configuration and message broadcasting.  
the test environment depend on the **zonky** postgres binary builds.


## Loading sequence:

- check database for existance and connect to DB
- create mem cache for settings. 
- create DB message listener/broadcaster.
- (*not implemented yet*) create local(./cache/...) cache for static resources 
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
dynamic module "_main" for executing the js module loading, serving responces, process messages.  
dynamic module "_transpile_dynamics" for transpiling jsqlx scripts into js.  


## database schema

### Settings table
Used to store global settings for all running instances
  
table: **nino_setting**
|: column |: type |: description |
|---------|-------|--------------|
| setting_key   | VARCHAR(256) PRIMARY KEY | the setting name  |
| setting_value | VARCHAR(4096)            | the setting value |
  
NINO is using the following settings to initialize the environment:  

|: setting |: default value |: description |
|---------|-------|--------------|
| nino_system_id | D01 | the nino environment identificator    |
| nino_core_thread_count | 3 |  the number of serving threads per instance   |
| nino_js_thread_count | 1 |  the number of JS V8 instances that will execute dynamic requests per instance    |
| nino_web_server_port | 8080 |  the serving port. same for all instances   |
| nino_db_connection_pool_size | 4 |  how many connections to keep in the connection pool   |
| nino_debug_port | 9229 |  the debug port for the instance. set 0 to disable the debugging. multiple  nino_js_thread_count will use sequential ports   |
| nino_web_request_timeout_ms | 10000 | the fetch default timeout from JavaScript    |



### Database connection table
Used to store connection string definitions for external databases.  
  
table: **nino_setting**
|: column |: type |: description |
|---------|-------|--------------|
| db_alias | VARCHAR(1024) PRIMARY KEY | database name that will be used in JavaScript   |
| db_type | VARCHAR(256) NOT NULL | currently only **postgres** type is supported   |
| db_connection_string | VARCHAR(4096) NOT NULL | the connection string used for the connection manager   |

The default connection is **_main** of type **postgres** and it is reserved name for the defailt db alias of the main application and points to the NINO environment value when executed.  
  

### Database connection table
Used to store connection string definitions for external databases.  
  
table: **nino_setting**
|: column |: type |: description |
|---------|-------|--------------|
| db_alias   | VARCHAR(256) PRIMARY KEY | the setting name  |
| setting_value | VARCHAR(4096)            | the setting value |



