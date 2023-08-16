The main loading sequence:
- check database for existance and connect to DB
- create mem cache for settings. 
- create local(./cache/...) cache for static and dynamic resources
- create DB message listener.
- create local static queue for serving threads based on the settings
- create local dynamic queue for serving threads based on the settings
- create local web server and stast pushing requests to sttic and dynamic queues

Components:

DBSyncManager - the sinc version to access the DB - used only to extract some settings for starting the environment.
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
to have a dynamic module "nino_main" for executing the dynamic executions.




DB
nino_setting - all settings related to the platform
  - setting_key
  - setting_value
  CREATE TABLE IF NOT EXISTS nino_settings (setting_key VARCHAR(256) PRIMARY KEY, setting_value VARCHAR(4096))

nino_request - contains all path mapings and the type of response behind:
  - path - /this/is/a/path
  - dynamic - to look into nino_static or nino_dynamic 
  - execute - to execute and return the result or to return the code itself
  - name - the path to look into nino_static or nino_dynamic 
  CREATE TABLE IF NOT EXISTS nino_request (path VARCHAR(1024) PRIMARY KEY, name VARCHAR(1024) NOT NULL, dynamic boolean, execute boolean, authorize boolean)

nino_static - all static resources behind
  - name - /this/is/a/path
  - mime
  - length
  - content 
  CREATE TABLE IF NOT EXISTS nino_static (name VARCHAR(1024) PRIMARY KEY, mime VARCHAR(64), NOT NULL, length INT, content BYTEA)

nino_dynamic - all dynamic code and modules are here
  - name - /module/will/be/loaded/from/here
  - code - the module code
  - js - transpiled code to js
  CREATE TABLE IF NOT EXISTS nino_dynamic (name VARCHAR(1024) PRIMARY KEY, code_length INT, js_length INT, code BYTEA, js BYTEA)