use crate::{db::DBManager, nino_constants, nino_structures};

#[derive(Clone)]
pub struct RequestManager {
    db: DBManager,
}

#[derive(Clone)]
pub struct RequestInfo {
    pub name: String,
    pub dynamic: bool,
    pub execute: bool,
    pub authorize: bool,
}

impl RequestManager {
    pub fn new(
        db: DBManager,
        db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::Message>,
    ) -> RequestManager {
        let this = Self { db };
        let thizz = this.clone();
        tokio::spawn(async move {
            thizz.invalidator(db_subscribe).await;
        });
        this
    }

    pub async fn invalidator(
        &self,
        mut db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::Message>,
    ) {
        loop {
            match db_subscribe.recv().await {
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                }
                Ok(message) => {
                    // todo! innvalidate cache
                    println!("web got message: {}", message.json);
                }
            }
        }
    }

    pub async fn get_request(&self, path: &str) -> Option<RequestInfo> {
        let query: String = format!(
            "SELECT name, dynamic, execute, authorize FROM {} WHERE path = $1",
            nino_constants::REQUESTS_TABLE
        );
        match self.db.query_one(&query, &[&path]).await {
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                None
            }
            Ok(row) => {
                let name: String = row.get(0);
                let dynamic: bool = row.get(1);
                let execute: bool = row.get(2);
                let authorize: bool = row.get(3);
                Some(RequestInfo {
                    name,
                    dynamic,
                    execute,
                    authorize,
                })
            }
        }
    }
}
