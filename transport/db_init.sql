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

-- nino_database is for storing DB connection strings
DROP TABLE IF EXISTS nino_database;
CREATE TABLE IF NOT EXISTS nino_database (
    db_alias VARCHAR(1024) PRIMARY KEY,
    db_type VARCHAR(256) NOT NULL,
    db_connection_string VARCHAR(4096) NOT NULL
);

INSERT INTO nino_database (db_alias, db_type, db_connection_string)
VALUES ('_main', 'postgres', 'reserved name for the defailt db alias of the main application');

-- nino_log
DROP TABLE IF EXISTS nino_log;
CREATE TABLE IF NOT EXISTS nino_log (
    log_timestamp TIMESTAMP DEFAULT NOW(),
    method VARCHAR(256) NOT NULL,
    request VARCHAR(1024),
    response VARCHAR(4096),
    log_message BYTEA
);
DROP INDEX IF EXISTS nino_log_ix;
CREATE INDEX IF NOT EXISTS nino_log_ix ON nino_log(log_timestamp);

-- request table for defining the requests
DROP TABLE IF EXISTS nino_request;
CREATE TABLE IF NOT EXISTS nino_request (
    request_path VARCHAR(1024) PRIMARY KEY,
    response_name VARCHAR(1024) NOT NULL,
    redirect_flag BOOLEAN DEFAULT FALSE,
    authorize_flag BOOLEAN DEFAULT FALSE
);

-- response table for defining static and dynamic requests
DROP TABLE IF EXISTS nino_response;
CREATE TABLE IF NOT EXISTS nino_response (
    response_name VARCHAR(1024) PRIMARY KEY,
    response_mime_type VARCHAR(256) NOT NULL,
    execute_flag BOOLEAN DEFAULT FALSE,
    transpile_flag BOOLEAN DEFAULT FALSE,
    response_content_length INT DEFAULT 0,
    response_content BYTEA NOT NULL, 
    javascript_length INT DEFAULT 0,
    javascript BYTEA
);

-- user and role tables
DROP TABLE IF EXISTS nino_user;
CREATE TABLE IF NOT EXISTS nino_user (
    user_name VARCHAR(256) PRIMARY KEY,
    user_password VARCHAR(256) NOT NULL
);
-- admin user
INSERT INTO nino_user (user_name, user_password) VALUES ('admin', '$2b$12$dGW/Xguize5gW1LrGBI3kOLy/vkk5jVwWuOqRPfLLaxCzYHHYhyGC');
INSERT INTO nino_user (user_name, user_password) VALUES ('user', '$2b$12$vetw61.n46JLD2Wc1njwreA/0UTyDxnHa6W73fliPJN9MJOZ1OHwO');

DROP TABLE IF EXISTS nino_role;
CREATE TABLE IF NOT EXISTS nino_role (
    user_role VARCHAR(256) PRIMARY KEY
);
INSERT INTO nino_role (user_role) VALUES ('admin');
INSERT INTO nino_role (user_role) VALUES ('user');

DROP TABLE IF EXISTS nino_user_role;
CREATE TABLE IF NOT EXISTS nino_user_role (
    user_name VARCHAR(256) NOT NULL,
    user_role VARCHAR(256) NOT NULL
);
DROP INDEX IF EXISTS nino_user_role_ix;
CREATE INDEX IF NOT EXISTS nino_user_role_ix ON nino_user_role(user_name);

INSERT INTO nino_user_role(user_name, user_role) VALUES ('admin', 'admin');
INSERT INTO nino_user_role(user_name, user_role) VALUES ('admin', 'portal');
INSERT INTO nino_user_role(user_name, user_role) VALUES ('admin', 'demo');
INSERT INTO nino_user_role(user_name, user_role) VALUES ('user', 'portal');
INSERT INTO nino_user_role(user_name, user_role) VALUES ('user', 'demo');

-- create portlet assigned to role and menu in the portal
-- sub menu is text separated by /
-- portlet index is used for ordering
DROP TABLE IF EXISTS nino_portlet;
CREATE TABLE IF NOT EXISTS nino_portlet (
    user_role VARCHAR(256) NOT NULL,
    portlet_menu VARCHAR(1024) NOT NULL,
    portlet_index INT DEFAULT 0,
    portlet_icon VARCHAR(256) NOT NULL,
    portlet_name VARCHAR(1024) NOT NULL
);
DROP INDEX IF EXISTS nino_portlet_ix;
CREATE INDEX IF NOT EXISTS nino_portlet_ix ON nino_portlet(user_role);


-- transports table with information
DROP TABLE IF EXISTS nino_transport;
CREATE TABLE IF NOT EXISTS nino_transport (
    transport_id          VARCHAR(64) NOT NULL,
    transport_status      VARCHAR(32) NOT NULL,
    transport_creator     VARCHAR(256) NOT NULL,
    transport_date        TIMESTAMP DEFAULT NOW(),
	transport_description TEXT
);
DROP INDEX IF EXISTS nino_transport_ix;
CREATE INDEX IF NOT EXISTS nino_transport_ix ON nino_transport(transport_id);


DROP TABLE IF EXISTS nino_transport_object;
CREATE TABLE IF NOT EXISTS nino_transport_object (
    transport_id           VARCHAR(64) NOT NULL,
    object_type            VARCHAR(32) NOT NULL,
    object_name            VARCHAR(512) NOT NULL,
    object_date            TIMESTAMP DEFAULT NOW(),
	object_description     TEXT,
	object_parameters_json TEXT NOT NULL,
	object_content         BYTEA NOT NULL,
	object_content_old     BYTEA NOT NULL
);
DROP INDEX IF EXISTS nino_transport_object_ix;
CREATE INDEX IF NOT EXISTS nino_transport_object_ix ON nino_transport_object(transport_id, object_type , object_name);
