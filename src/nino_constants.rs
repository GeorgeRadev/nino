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

macro_rules! info {
    ($($arg:tt)*) => (if ::std::cfg!(debug_assertions) { ::std::println!($($arg)*); })
}
pub(crate) use info;

/// program name
pub const PROGRAM_NAME: &str = PKG_NAME!();
pub const PROGRAM_VERSION: &str = PKG_VERSION!();

/// use cache parameters
pub static USE_REQUEST_CACHE: bool = true;
pub static USE_RESPONSE_CACHE: bool = true;

/// SETTINGS table name
pub const SETTINGS_TABLE: &str = concat!(PKG_NAME!(), "_setting");
/// REQUESTS table name
pub const REQUESTS_TABLE: &str = concat!(PKG_NAME!(), "_request");
/// RESPONSE table name
pub const RESPONSE_TABLE: &str = concat!(PKG_NAME!(), "_response");
/// DATABASE table name with connection strings
pub const DATABASE_TABLE: &str = concat!(PKG_NAME!(), "_database");
/// LOG table
pub const LOG_TABLE: &str = concat!(PKG_NAME!(), "_log");

/// JS settings
///
pub const MODULE_URI: &str = "http://nino.db/";
pub const MAIN_MODULE: &str = "_main";
pub const TRANSPILE_MODULE: &str = "_transpile_dynamics";
pub const MAIN_DB: &str = "_main";
pub const DB_TYPE_POSTGRES: &str = "postgres";
pub const JWT_USER: &str = "user";

/// SETTINGS table constants

/// defines the system name usually: XNN
/// X = D-Devlopment, Q-Quality, S-Support, R-Regression, P-Production
pub const SETTINGS_NINO_SYSTEM_ID: &str = "nino_system_id";
pub const SETTINGS_NINO_SYSTEM_ID_DEFAULT: &str = "N1N0";

/// defines how many threads the system will set in tokio
pub const SETTINGS_NINO_THREAD_COUNT: &str = "nino_core_thread_count";
pub const SETTINGS_NINO_THREAD_COUNT_DEFAULT: i32 = 4;

/// defines how many JavaScript threads to serve requests
pub const SETTINGS_JS_THREAD_COUNT: &str = "nino_js_thread_count";
pub const SETTINGS_JS_THREAD_COUNT_DEFAULT: i32 = 1;

/// defines web serving port
pub const SETTINGS_NINO_WEB_SERVER_PORT: &str = "nino_web_server_port";
pub const SETTINGS_NINO_WEB_SERVER_PORT_DEFAULT: i32 = 8080;

/// defines how many connections to be in each connection pool
pub const SETTINGS_DB_CONNECTION_POOL_SIZE: &str = "nino_db_connection_pool_size";
pub const SETTINGS_DB_CONNECTION_POOL_SIZE_DEFAULT: i32 = 0;

/// debug port
pub const SETTINGS_NINO_DEBUG_PORT: &str = "nino_debug_port";
pub const SETTINGS_NINO_DEBUG_PORT_DEFAULT: i32 = 0;

/// defines web request timeout
pub const SETTINGS_NINO_WEB_REQUEST_TIMEOUT: &str = "nino_web_request_timeout_ms";
pub const SETTINGS_NINO_WEB_REQUEST_TIMEOUT_DEFAULT: i32 = 30000;

/// login servlet - gets the original path in the query
pub const SETTINGS_NINO_LOGIN_PATH: &str = "nino_login_paths";
pub const SETTINGS_NINO_LOGIN_PATH_DEFAULT: &str = "/login";