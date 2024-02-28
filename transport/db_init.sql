-- nino_database is for storing DB connection strings
DROP TABLE IF EXISTS nino_database;
CREATE TABLE IF NOT EXISTS nino_database (db_alias VARCHAR(1024) PRIMARY KEY, db_type VARCHAR(256) NOT NULL, db_connection_string VARCHAR(4096) NOT NULL);
INSERT INTO nino_database (db_alias, db_type, db_connection_string)VALUES ('_main', 'postgres', 'reserved name for the defailt db alias of the main application');
INSERT INTO nino_database (db_alias, db_type, db_connection_string) VALUES ('test', 'postgres', 'postgresql://dead_one@localhost/postgres?connect_timeout=10');

-- nino_setting default environment parameters
DROP TABLE IF EXISTS nino_setting;
CREATE TABLE IF NOT EXISTS nino_setting (setting_key VARCHAR(256) PRIMARY KEY, setting_value VARCHAR(4096));
INSERT INTO nino_setting (setting_key, setting_value) VALUES ('nino_core_thread_count','3');
INSERT INTO nino_setting (setting_key, setting_value) VALUES ('nino_js_thread_count','1');
INSERT INTO nino_setting (setting_key, setting_value) VALUES ('nino_web_server_port','8080');
INSERT INTO nino_setting (setting_key, setting_value) VALUES ('nino_db_connection_pool_size','4');
INSERT INTO nino_setting (setting_key, setting_value) VALUES ('nino_debug_port','9229');
INSERT INTO nino_setting (setting_key, setting_value) VALUES ('nino_web_request_timeout_ms','10000');

-- request tables for static and dynamic code
DROP TABLE IF EXISTS nino_request;
CREATE TABLE IF NOT EXISTS nino_request (path VARCHAR(1024) PRIMARY KEY, name VARCHAR(1024) NOT NULL, mime VARCHAR(64) NOT NULL, redirect boolean, authorize boolean, dynamic boolean, execute boolean);
DROP TABLE IF EXISTS nino_static;
CREATE TABLE IF NOT EXISTS nino_static (name VARCHAR(1024) PRIMARY KEY, content_length INT, content BYTEA);
DROP TABLE IF EXISTS nino_dynamic;
CREATE TABLE IF NOT EXISTS nino_dynamic (name VARCHAR(1024) PRIMARY KEY, code_length INT, js_length INT, transpile boolean, code BYTEA, js BYTEA);

-- user and role tables
DROP TABLE IF EXISTS nino_user;
CREATE TABLE IF NOT EXISTS nino_user (username VARCHAR(256) PRIMARY KEY, password VARCHAR(256) NOT NULL);
INSERT INTO nino_user (username, password) VALUES ('admin', '$2b$12$dGW/Xguize5gW1LrGBI3kOLy/vkk5jVwWuOqRPfLLaxCzYHHYhyGC');

DROP TABLE IF EXISTS nino_user_role;
CREATE TABLE IF NOT EXISTS nino_user_role (username VARCHAR(256) NOT NULL, user_role VARCHAR(256) NOT NULL);
INSERT INTO nino_user_role (username, user_role) VALUES ('admin', '/about/');
INSERT INTO nino_user_role (username, user_role) VALUES ('admin', '/test/');

DROP TABLE IF EXISTS nino_role_portlet;
CREATE TABLE IF NOT EXISTS nino_role_portlet (user_role VARCHAR(256) NOT NULL, portlet VARCHAR(256) NOT NULL);
INSERT INTO nino_role_portlet (user_role, portlet) VALUES ('/about/','portlet_about.js');
INSERT INTO nino_role_portlet (user_role, portlet) VALUES ('/test/','portlet_test.js');

