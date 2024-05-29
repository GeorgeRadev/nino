use crate::db::DBManager;
use crate::{db_notification, nino_constants, nino_structures};
use deno_runtime::deno_core::anyhow::Error;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// A Postgres DB connector and listener
/// plus a connection pool for executing transaction
#[derive(Clone)]
pub struct SettingsManager {
    db: Arc<DBManager>,
}

#[derive(Clone)]
struct CachedValue {
    str: String,
    int: Option<i32>,
}

static SETTING_CACHE: OnceLock<RwLock<HashMap<String, CachedValue>>> = OnceLock::new();

impl SettingsManager {
    /// Create DB Manager and connection pool
    pub fn new(
        db: Arc<DBManager>,
        db_subscribe: Option<
            tokio::sync::broadcast::Receiver<nino_structures::NotificationMessage>,
        >,
    ) -> SettingsManager {
        SETTING_CACHE.get_or_init(|| RwLock::new(HashMap::new()));
        if let Some(subscribe) = db_subscribe {
            tokio::spawn(async move {
                Self::invalidator(subscribe).await;
            });
        }
        Self { db }
    }

    pub async fn invalidator(
        mut db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::NotificationMessage>,
    ) {
        loop {
            match db_subscribe.recv().await {
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
                Ok(message) => {
                    println!("MSG:settings: {}", message.text);
                    if message
                        .text
                        .starts_with(db_notification::NOTIFICATION_PREFIX_SETTING)
                    {
                        SETTING_CACHE.get().unwrap().write().unwrap().clear();
                    }
                }
            }
        }
    }

    fn cache_get(settings_key: &str) -> Option<CachedValue> {
        // check if setting is in the cache
        SETTING_CACHE
            .get()
            .unwrap()
            .read()
            .unwrap()
            .get(settings_key)
            .cloned()
    }

    fn cache_set(settings_key: &str, value: CachedValue) {
        // cache value
        SETTING_CACHE
            .get()
            .unwrap()
            .write()
            .unwrap()
            .insert(settings_key.into(), value);
    }

    async fn get_setting(&self, settings_key: &str) -> Result<Option<CachedValue>, Error> {
        if let Some(value) = Self::cache_get(settings_key) {
            return Ok(Some(value));
        }
        let query = format!(
            "SELECT setting_value FROM {} WHERE setting_key = $1",
            nino_constants::SETTINGS_TABLE
        );

        match self.db.query_opt(&query, &[&settings_key]).await? {
            Some(row) => {
                let str: String = row.get(0);
                let int = match str.parse::<i32>() {
                    Ok(v) => Some(v),
                    Err(_) => None,
                };
                let value = CachedValue { str, int };
                Self::cache_set(settings_key, value.clone());
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    pub async fn get_setting_str(&self, settings_key: &str, def_value: &str) -> String {
        match self.get_setting(settings_key).await {
            Ok(value) => match value {
                Some(value) => value.str,
                None => def_value.to_owned(),
            },
            Err(_) => def_value.to_owned(),
        }
    }

    pub async fn get_setting_i32(&self, settings_key: &str, def_value: i32) -> i32 {
        match self.get_setting(settings_key).await {
            Ok(value) => match value {
                Some(value) => match value.int {
                    Some(v) => v,
                    None => def_value,
                },
                None => def_value,
            },
            Err(_) => def_value,
        }
    }

    pub async fn get_setting_usize(&self, settings_key: &str, def_value: i32) -> usize {
        let v = self.get_setting_i32(settings_key, def_value).await;
        v as usize
    }
}
