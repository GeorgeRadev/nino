use crate::nino_structures::LogInfo;
use crate::{db::DBManager, nino_constants};
use async_channel::{Receiver, Sender};
use std::sync::{Arc, OnceLock};

/// A loggerManagerre message and context info into the db log table
pub struct DBLogger {}

static LOG_CHANNEL: OnceLock<Sender<LogInfo>> = OnceLock::new();

impl DBLogger {
    /// Create DB Manager and connection pool
    pub fn new(db: Arc<DBManager>) -> DBLogger {
        let (log_sx, log_rx) = async_channel::unbounded::<LogInfo>();
        LOG_CHANNEL.get_or_init(|| log_sx);
        //spawn loging async task
        tokio::spawn(DBLogger::log_loop(db, log_rx));
        // config = connection_string.parse::<Config>().unwrap();
        DBLogger {}
    }

    pub async fn log(log: LogInfo) {
        match LOG_CHANNEL.get() {
            Some(log_sx) => {
                if let Err(error) = log_sx.send(log).await {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
            }
            None => panic!("Please call DBLogger::new to initialize async channels for logging"),
        }
    }

    pub async fn log_loop(db: Arc<DBManager>, log_rx: Receiver<LogInfo>) {
        loop {
            match log_rx.recv().await {
                Ok(log) => {
                    //log into db
                    let query: String =
                        format!("INSERT INTO {} (method, request, response, log_message) VALUES ($1, $2, $3, $4)", nino_constants::LOG_TABLE);
                    if let Err(error) = db
                        .execute(
                            &query,
                            &[&log.method, &log.request, &log.response, &log.message.as_bytes()],
                        )
                        .await
                    {
                        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                    }
                }
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
            }
        }
    }
}
