use deadpool_postgres::{Manager, ManagerConfig, Object, Pool, RecyclingMethod};
use deno_core::anyhow::Error;
use tokio_postgres::types::ToSql;
use tokio_postgres::{Config, Row};

/// A Postgres DB connector and listener
/// plus a connection pool for executing transaction
pub struct DBManager {
    connection_string: String,
    pool: Pool,
}

impl DBManager {
    /// Create DB Manager and connection pool
    pub async fn instance(connection_string: String, pool_size: usize) -> Result<DBManager, Error> {
        let config = connection_string.parse::<Config>().unwrap();
        let db = config.connect(tokio_postgres::NoTls).await?;
        let (_client, connection) = db;
        //spawn connector for notifications
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            }
        });

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let mgr = Manager::from_config(config, tokio_postgres::NoTls, mgr_config);
        let pool = Pool::builder(mgr).max_size(pool_size).build().unwrap();

        // config = connection_string.parse::<Config>().unwrap();
        Ok(DBManager {
            connection_string,
            pool,
        })
    }

    pub fn get_connection_string(&self) -> String {
        self.connection_string.clone()
    }

    pub async fn get_connection(&self) -> Result<Object, Error> {
        self.pool.get().await.map_err(|e| e.into())
    }

    pub async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        let db = self.get_connection().await?;
        db.execute(query, params).await.map_err(|e| e.into())
    }

    pub async fn query(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, Error> {
        let db = self.get_connection().await?;
        db.query(query, params).await.map_err(|e| e.into())
    }

    pub async fn query_opt(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error> {
        let db = self.get_connection().await?;
        db.query_opt(query, params).await.map_err(|e| e.into())
    }
}
