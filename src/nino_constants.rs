macro_rules! PKG_NAME {
    () => {
        env!("CARGO_PKG_NAME")
    };
}

macro_rules! PKG_VERSION {
    () => {
        env!("CARGO_PKG_VERSION")
    };
}

///program name
pub const PROGRAM_NAME: &str = PKG_NAME!();
pub const PROGRAM_VERSION: &str = PKG_VERSION!();

/// SETTINGS table name
pub const SETTINGS_TABLE: &str = concat!(PKG_NAME!(), "_setting");
/// REQUESTS table name
pub const REQUESTS_TABLE: &str = concat!(PKG_NAME!(), "_request");
/// STATICS table name
pub const STATICS_TABLE: &str = concat!(PKG_NAME!(), "_static");
/// DYNAMICS table name
pub const DYNAMICS_TABLE: &str = concat!(PKG_NAME!(), "_dynamic");

/// JS settings
/// 
pub const MODULE_URI: &str = "http://nino.db/";
pub const MAIN_MODULE: &str = "_main";

/// SETTINGS table constants

/// defines how many threads the system will set in tokio
pub const SETTINGS_NINO_THREAD_COUNT: &str = "nino_core_thread_count";
pub const SETTINGS_NINO_THREAD_COUNT_DEFAULT: i32 = 4;

/// defines how many JavaScript threads to serve requests
pub const SETTINGS_JS_THREAD_COUNT: &str = "nino_js_thread_count";
pub const SETTINGS_JS_THREAD_COUNT_DEFAULT: i32 = 4;

/// defines web serving port
pub const SETTINGS_NINO_WEB_SERVER_PORT: &str = "nino_web_server_port";
pub const SETTINGS_NINO_WEB_SERVER_PORT_DEFAULT: i32 = 8080;

/// defines how many connections to be in each connection pool
pub const SETTINGS_DB_CONNECTION_POOL_SIZE: &str = "nino_db_connection_pool_size";
pub const SETTINGS_DB_CONNECTION_POOL_SIZE_DEFAULT: i32 = 4;

/// debug port
pub const SETTINGS_NINO_DEBUG_PORT: &str = "nino_debug_port";
pub const SETTINGS_NINO_DEBUG_PORT_DEFAULT: i32 = 0;



