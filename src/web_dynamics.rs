use crate::{
    db::DBManager,
    nino_constants,
    nino_structures::{self, WebTask},
};
use async_channel::{Receiver, Sender};
use async_std::net::TcpStream;
use http_types::Request;

#[derive(Clone)]
pub struct DynamicsManager {
    db: DBManager,
    web_task_sx: Sender<Box<nino_structures::WebTask>>,
    web_task_rx: Receiver<Box<nino_structures::WebTask>>,
    module_sx: tokio::sync::mpsc::UnboundedSender<(
        deno_core::ModuleSpecifier,
        tokio::sync::mpsc::Sender<String>,
    )>,
}

impl DynamicsManager {
    pub fn new(
        db: DBManager,
        db_subscribe: tokio::sync::broadcast::Receiver<nino_structures::Message>,
    ) -> DynamicsManager {
        let (web_task_sx, web_task_rx) =
            async_channel::unbounded::<Box<nino_structures::WebTask>>();
        let (module_sx, mut module_rx) = tokio::sync::mpsc::unbounded_channel::<(
            deno_core::ModuleSpecifier,
            tokio::sync::mpsc::Sender<String>,
        )>();
        let this = Self {
            db,
            web_task_sx,
            web_task_rx,
            module_sx,
        };
        let thizz = this.clone();
        tokio::spawn(async move {
            thizz.invalidator(db_subscribe).await;
        });
        let thizzz = this.clone();
        tokio::spawn(async move {
            loop {
                match module_rx.recv().await {
                    Some((path, sender)) => {
                        let path = crate::nino_functions::normalize_path(path.path().to_string());
                        let mut code = thizzz.get_module_js(&path).await;
                        if code.is_none() {
                            code = Some(String::from(""));
                        }
                        loop {
                            if sender.send(code.clone().unwrap()).await.is_ok() {
                                break;
                            }
                        }
                    }
                    None => {}
                }
            }
        });
        // let (sender, receiver) = spmc::channel::<String>();
        // let _r = this.get_module_js_channel(String::from("test_servlet"), sender);
        // let code = receiver.recv();
        // eprintln!("code: {}", code.unwrap());
        this
    }

    pub fn get_web_task_rx(&self) -> Receiver<Box<WebTask>> {
        self.web_task_rx.clone()
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
                    println!("got message: {}", message.json);
                }
            }
        }
    }

    // returns the longest matching path
    async fn get_matching_path(&self, path: &str) -> Option<String> {
        let query: String = format!(
            "SELECT name FROM {} WHERE name = $1",
            nino_constants::DYNAMICS_TABLE
        );
        match self.db.query(&query, &[&path]).await {
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                None
            }
            Ok(rows) => {
                for row in rows {
                    let path: String = row.get(0);
                    if path.len() > 0 {
                        return Some(path);
                    }
                }
                None
            }
        }
    }

    // returns the longest matching path
    pub async fn get_module_js(&self, path: &str) -> Option<String> {
        let query: String = format!(
            "SELECT js FROM {} WHERE name = $1",
            nino_constants::DYNAMICS_TABLE
        );
        let r = self.db.query_one(&query, &[&path]).await;
        match r {
            Err(error) => {
                eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            }
            Ok(row) => {
                let js_bytes: Vec<u8> = row.get(0);
                let js = String::from_utf8(js_bytes).unwrap();
                if js.len() > 0 {
                    return Some(js);
                }
            }
        }
        None
    }

    pub async fn serve_dynamic(
        &self,
        path: &str,
        request: Request,
        stream: Box<TcpStream>,
    ) -> bool {
        // look for matching path
        if let Some(js_module) = self.get_matching_path(path).await {
            //send new task to the javascript threads
            let web_task = Box::new(nino_structures::WebTask {
                js_module,
                request,
                stream,
            });
            match self.web_task_sx.send(web_task).await {
                Ok(_) => true,
                Err(error) => {
                    eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
                    //return error
                    false
                }
            }
        } else {
            false
        }
    }
}
