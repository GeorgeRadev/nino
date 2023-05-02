use crate::{db::DBManager, nino_structures};

macro_rules! PKG_NAME {
    () => {
        env!("CARGO_PKG_NAME")
    };
}

/// A Postgres DB connector and listener
/// plus a connection pool for executing transaction
pub struct DBNotificationManager {
    //listener: std::thread::JoinHandle<bool>,
    broadcast_sx: tokio::sync::broadcast::Sender<nino_structures::Message>,
    db: DBManager,
}

impl DBNotificationManager {
    /// Create a thread for listening broadcast notifications
    pub fn new(db: DBManager) -> DBNotificationManager {
        // 1 - * subscribe message - broadcast messages from DB broadcast
        let (broadcast_sx, _broadcast_rx) =
            tokio::sync::broadcast::channel::<nino_structures::Message>(64);

        let _listener;
        {
            // spawn listener
            let cs = db.get_connection_string();
            let broadcast_sender = broadcast_sx.clone();
            _listener = std::thread::Builder::new()
                .name("notification listener".to_string())
                .spawn(move || Self::start_listening_for_messages(&cs, broadcast_sender));
        }

        DBNotificationManager {
            //listener,
            broadcast_sx,
            db: db,
        }
    }

    /// get subscriper channel for recieving notifications
    pub fn get_subscriber(&self) -> tokio::sync::broadcast::Receiver<nino_structures::Message> {
        self.broadcast_sx.subscribe()
    }

    /// send message to all (local and global) subscribers through the db
    pub async fn notify(&mut self, msg: String) -> Result<u64, String> {
        let db = self.db.get_connection().await?;
        let statement = format!("NOTIFY {}, {}", PKG_NAME!(), escape_single_quotes(&msg));
        let result = db.execute(&statement, &[]).await;
        result.map_err(|e| e.to_string())
    }

    fn start_listening_for_messages(
        connection_string: &String,
        broadcast_sx: tokio::sync::broadcast::Sender<nino_structures::Message>,
    ) -> ! {
        use postgres::fallible_iterator::FallibleIterator;
        use std::time::Duration;
        loop {
            let mut db = match postgres::Client::connect(&connection_string, postgres::NoTls) {
                Ok(db) => db,
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                    std::thread::sleep(Duration::from_millis(5000));
                    continue;
                }
            };
            let mut _r = match db.execute(concat!("LISTEN ", PKG_NAME!()), &[]) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                    std::thread::sleep(Duration::from_millis(5000));
                    continue;
                }
            };

            let mut notifications = db.notifications();
            loop {
                let mut it = notifications.blocking_iter();
                let msg_result = it.next();
                match msg_result {
                    Ok(msg) => match msg {
                        Some(c) => {
                            println!("message: {:?}", c);
                            // broad cast message to listeners
                            if let Err(error) = broadcast_sx.send(nino_structures::Message {
                                json: c.payload().to_string(),
                            }) {
                                eprintln!(
                                    "ERROR {}:{}:sending message {}",
                                    file!(),
                                    line!(),
                                    error
                                );
                            }
                        }
                        _ => {
                            // no message
                        }
                    },
                    Err(error) => {
                        eprintln!("ERROR {}:{}:sending message {}", file!(), line!(), error);
                        break;
                    }
                }
            }
        }
    }
}

fn escape_single_quotes(value: &str) -> String {
    let quote = '\'';
    let len = value.len();
    if len <= 0 {
        return "NULL".to_string();
    }
    let mut result = Vec::with_capacity(2 + 2 * len);
    result.push(quote);
    for c in value.chars() {
        if c == quote {
            result.push(c);
        }
        result.push(c);
    }
    result.push(quote);

    result.iter().collect()
}
