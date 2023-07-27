use deadpool_postgres::{Manager, ManagerConfig, Object, Pool, RecyclingMethod};
use std::sync::Arc;
use tokio_postgres::types::ToSql;
use tokio_postgres::{Config, Row};

/// A Postgres DB connector and listener
/// plus a connection pool for executing transaction
#[derive(Clone)]
pub struct DBManager {
    connection_string: String,
    pool: std::sync::Arc<Pool>,
}

impl DBManager {
    /// Create DB Manager and connection pool
    pub async fn instance(
        connection_string: String,
        pool_size: usize,
    ) -> Result<DBManager, String> {
        let config = connection_string.parse::<Config>().unwrap();
        let db = config.connect(tokio_postgres::NoTls).await;
        if db.is_err() {
            let err_str = format!(
                "ERROR:{}:{}:{}",
                file!(),
                line!(),
                db.err().unwrap()
            );
            eprintln!("{}", err_str);
            return Err(err_str);
        }
        let (_client, _connection) = db.unwrap();
        //spawn connector for notifications
        // tokio::spawn(async move {
        //     if let Err(error) = connection.await {
        //         eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
        //     }
        // });

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let mgr = Manager::from_config(config, tokio_postgres::NoTls, mgr_config);
        let pool = Pool::builder(mgr).max_size(pool_size).build().unwrap();

        // config = connection_string.parse::<Config>().unwrap();
        Ok(DBManager {
            connection_string,
            pool: Arc::new(pool),
        })
    }

    pub fn get_connection_string(&self) -> String {
        self.connection_string.clone()
    }

    pub async fn get_connection(&self) -> Result<Object, String> {
        self.pool.clone().get().await.map_err(|e| e.to_string())
    }

    pub async fn execute(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<u64, String> {
        let db = self.get_connection().await.map_err(|e| e.to_string())?;
        db.execute(query, params).await.map_err(|e| e.to_string())
    }

    pub async fn query(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, String> {
        let db = self.get_connection().await.map_err(|e| e.to_string())?;
        db.query(query, params).await.map_err(|e| e.to_string())
    }

    pub async fn query_one(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, String> {
        let db = self.get_connection().await.map_err(|e| e.to_string())?;
        db.query_one(query, params).await.map_err(|e| e.to_string())
    }
    /*
    pub async fn query_callback(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
        callback: &dyn Fn(u32, Row) -> bool,
    ) -> Result<u32, String> {
        let db = self.get_connection().await.map_err(|e| e.to_string())?;

        let mut count: u32 = 0;
        for row in db.query(query, params).await.map_err(|e| e.to_string())? {
            callback(count, row);
            count += 1;
        }
        Ok(count)
    }
    */
}
