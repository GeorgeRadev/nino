-- nino_database is for storing DB connection strings
DROP TABLE IF EXISTS nino_database;
CREATE TABLE IF NOT EXISTS nino_database (
    db_alias VARCHAR(1024) PRIMARY KEY,
    db_type VARCHAR(256) NOT NULL,
    db_connection_string VARCHAR(4096) NOT NULL
);

INSERT INTO nino_database (db_alias, db_type, db_connection_string)
VALUES ('_main', 'postgres', 'reserved name for the defailt db alias of the main application');

-- nino_setting default environment parameters
DROP TABLE IF EXISTS nino_setting;

CREATE TABLE IF NOT EXISTS nino_setting (
    setting_key VARCHAR(256) PRIMARY KEY,
    setting_value VARCHAR(4096)
);

INSERT INTO nino_setting (setting_key, setting_value) VALUES ('nino_system_id','D01');
INSERT INTO nino_setting (setting_key, setting_value) VALUES ('nino_core_thread_count','3');
INSERT INTO nino_setting (setting_key, setting_value) VALUES ('nino_js_thread_count','1');
INSERT INTO nino_setting (setting_key, setting_value) VALUES ('nino_web_server_port','8080');
INSERT INTO nino_setting (setting_key, setting_value) VALUES ('nino_db_connection_pool_size','4');
INSERT INTO nino_setting (setting_key, setting_value) VALUES ('nino_debug_port','9229');
INSERT INTO nino_setting (setting_key, setting_value) VALUES ('nino_web_request_timeout_ms','10000');

-- request tables for static and dynamic code
DROP TABLE IF EXISTS nino_request;
CREATE TABLE IF NOT EXISTS nino_request (
    request_path VARCHAR(1024) PRIMARY KEY,
    request_name VARCHAR(1024) NOT NULL,
    request_mime_type VARCHAR(64) NOT NULL,
    redirect_flag BOOLEAN DEFAULT FALSE,
    authorize_flag BOOLEAN DEFAULT FALSE, 
    dynamic_flag BOOLEAN DEFAULT FALSE,
    execute_flag BOOLEAN DEFAULT FALSE
);

DROP TABLE IF EXISTS nino_static;
CREATE TABLE IF NOT EXISTS nino_static (
    static_name VARCHAR(1024) PRIMARY KEY,
    static_content_length INT NOT NULL,
    static_content BYTEA NOT NULL
);

DROP TABLE IF EXISTS nino_dynamic;
CREATE TABLE IF NOT EXISTS nino_dynamic (
    dynamic_name VARCHAR(1024) PRIMARY KEY,
    code BYTEA NOT NULL,
    code_length INT DEFAULT 0, 
    transpile_flag BOOLEAN DEFAULT FALSE,
    javascript BYTEA,
    javascript_length INT DEFAULT 0
);

-- user and role tables
DROP TABLE IF EXISTS nino_user;
CREATE TABLE IF NOT EXISTS nino_user (
    user_name VARCHAR(256) PRIMARY KEY,
    user_password VARCHAR(256) NOT NULL
);
-- admin user
INSERT INTO nino_user (user_name, user_password) VALUES ('admin', '$2b$12$dGW/Xguize5gW1LrGBI3kOLy/vkk5jVwWuOqRPfLLaxCzYHHYhyGC');

DROP TABLE IF EXISTS nino_role;
CREATE TABLE IF NOT EXISTS nino_role (
    user_role VARCHAR(256) PRIMARY KEY
);
INSERT INTO nino_role (user_role) VALUES ('admin');

DROP TABLE IF EXISTS nino_user_role;
CREATE TABLE IF NOT EXISTS nino_user_role (
    user_name VARCHAR(256) NOT NULL,
    user_role VARCHAR(256) NOT NULL
);
DROP INDEX IF EXISTS nino_user_role_ix;
CREATE INDEX IF NOT EXISTS nino_user_role_ix ON nino_user_role(user_name);

INSERT INTO nino_user_role(user_name, user_role) VALUES ('admin', 'admin');

-- create portlet assigned to role and menu in the portal
-- sub menu is text separated by /
-- portlet index is used for ordering
DROP TABLE IF EXISTS nino_portlet;
CREATE TABLE IF NOT EXISTS nino_portlet (
    user_role VARCHAR(256) NOT NULL,
    portlet_menu VARCHAR(1024) NOT NULL,
    portlet_index INT DEFAULT 0,
    portlet_name VARCHAR(1024) NOT NULL
);
DROP INDEX IF EXISTS nino_portlet_ix;
CREATE INDEX IF NOT EXISTS nino_portlet_ix ON nino_portlet(user_role);
