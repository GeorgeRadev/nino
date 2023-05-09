use crate::db::DBManager;
use crate::nino_constants;

/// A Postgres DB connector and listener
/// plus a connection pool for executing transaction
pub struct SettingsManager {
    db: DBManager,
}

impl Clone for SettingsManager {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
        }
    }
}

impl SettingsManager {
    /// Create DB Manager and connection pool
    pub fn new(db: DBManager) -> SettingsManager {
        Self { db }
    }

    async fn get_setting(&self, settings_key: &str) -> Result<String, String> {
        let query = format!(
            "SELECT setting_value FROM {} WHERE setting_key = $1",
            nino_constants::SETTINGS_TABLE
        );

        match self.db.query_one(&query, &[&settings_key]).await {
            Ok(row) => {
                let value: String = row.get(0);
                Ok(value)
            }
            Err(error) => {
                eprintln!("WARNING missing setting for key '{}' :{}:{}:{}", settings_key, file!(), line!(), error);
                Err(error.to_string())
            }
        }
    }

    pub async fn get_setting_i32(&self, settings_key: &str, def_value: i32) -> i32 {
        match self.get_setting(settings_key).await {
            Ok(value_string) => {
                if value_string.len() > 0 {
                    match value_string.parse::<i32>() {
                        Ok(v) => v,
                        Err(_) => def_value,
                    }
                } else {
                    def_value
                }
            }
            Err(_) => def_value,
        }
    }

    pub async fn get_setting_usize(&self, settings_key: &str, def_value: i32) -> usize {
        let v = self.get_setting_i32(settings_key, def_value).await;
        v as usize
    }

    pub async fn get_server_port(&self) -> u16 {
        self.get_setting_i32(
            nino_constants::SETTINGS_NINO_WEB_SERVER_PORT,
            nino_constants::SETTINGS_NINO_WEB_SERVER_PORT_DEFAULT,
        )
        .await as u16
    }

    pub async fn get_thread_count(&self) -> usize {
        self.get_setting_usize(
            nino_constants::SETTINGS_NINO_THREAD_COUNT,
            nino_constants::SETTINGS_NINO_THREAD_COUNT_DEFAULT,
        )
        .await
    }

    pub async fn get_db_pool_size(&self) -> usize {
        self.get_setting_usize(
            nino_constants::SETTINGS_DB_CONNECTION_POOL_SIZE,
            nino_constants::SETTINGS_DB_CONNECTION_POOL_SIZE_DEFAULT,
        )
        .await
    }

    pub async fn get_js_thread_count(&self) -> u16 {
        self.get_setting_i32(
            nino_constants::SETTINGS_JS_THREAD_COUNT,
            nino_constants::SETTINGS_JS_THREAD_COUNT_DEFAULT,
        )
        .await as u16
    }

    pub async fn get_debug_port(&self) -> u16 {
        self.get_setting_i32(
            nino_constants::SETTINGS_NINO_DEBUG_PORT,
            nino_constants::SETTINGS_NINO_DEBUG_PORT_DEFAULT,
        )
        .await as u16
    }
}
