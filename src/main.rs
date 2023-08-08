mod db;
mod db_notification;
mod db_transactions;
mod db_settings;
mod js;
mod js_functions;
mod js_test_debug;
mod js_test;
mod nino_constants;
mod nino_functions;
mod nino_structures;
mod trasport;
mod web;
mod web_dynamics;
mod web_requests;
mod web_statics;

use nino_structures::InitialSettings;
use std::sync::Arc;

use crate::db_transactions::TransactionManager;

fn main() {
    setup_panic_hook();

    let connection_string = nino_functions::get_connection_string()
        .map_err(|_| help())
        .unwrap();

    // get starting-up settings
    let initial_settings = {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(get_db_settings(connection_string))
    };

    // async functionalities goes here
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(initial_settings.thread_count)
        .enable_all()
        .build()
        .unwrap()
        .block_on(main_async(initial_settings));
}

macro_rules! await_and_exit_on_error {
    ($future:expr) => {
        if let Err(error) = $future.await {
            eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
            std::process::exit(1);
        }
    };
}

async fn get_db_settings(connection_string: String) -> InitialSettings {
    let db;
    loop {
        match db::DBManager::instance(connection_string.clone(), 2).await {
            Ok(inst) => {
                db = inst;
                break;
            }
            Err(error) => eprintln!("Waiting for database...({})", error),
        };
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    eprintln!("Database ok");

    let settings = db_settings::SettingsManager::new(db);
    let thread_count = settings.get_thread_count().await;
    let js_thread_count = settings.get_js_thread_count().await;
    let server_port = settings.get_server_port().await;
    let debug_port = settings.get_debug_port().await;
    let mut db_pool_size = settings.get_db_pool_size().await;
    if db_pool_size == 0 {
        // match db pool to serving threads + js threads
        db_pool_size = thread_count + js_thread_count;
    }

    InitialSettings {
        connection_string: connection_string.clone(),
        thread_count,
        server_port,
        debug_port,
        db_pool_size,
        js_thread_count,
    }
}

async fn main_async(settings: InitialSettings) {
    let db = Arc::new(
        db::DBManager::instance(settings.connection_string.clone(), settings.db_pool_size)
            .await
            .unwrap(),
    );

    let db_notifier = db_notification::DBNotificationManager::new(db.clone());
    {
        // transport initial db content
        let transport = trasport::TransportManager::new(db.clone());
        await_and_exit_on_error!(transport.transport_file("./transports/0_transport.json"));
    }

    let requests = Arc::new(web_requests::RequestManager::new(
        db.clone(),
        db_notifier.get_subscriber(),
    ));

    let statics = Arc::new(web_statics::StaticManager::new(
        db.clone(),
        db_notifier.get_subscriber(),
    ));

    let dyn_subscriber = db_notifier.get_subscriber();
    let notifier = Arc::new(db_notification::Notifier::new(Arc::new(db_notifier)));

    let dynamics = Arc::new(web_dynamics::DynamicManager::new(
        db.clone(),
        settings.js_thread_count,
        notifier.clone(),
        dyn_subscriber,
    ));

    let tx_sessions = Arc::new(TransactionManager::instance(settings.js_thread_count, db.clone()));

    let _js_engine = js::JavaScriptManager::instance(
        settings.js_thread_count,
        settings.debug_port,
        Some(db.clone()),
        Some(dynamics.clone()),
        Some(tx_sessions),
    );
    // start js threads
    js::JavaScriptManager::start().await;

    await_and_exit_on_error!(notifier.notify("string message".to_string()));

    let web = web::WebManager::new(
        settings.server_port,
        requests.clone(),
        statics.clone(),
        dynamics.clone(),
    );
    await_and_exit_on_error!(web.start());
}

/// prints execution parameter information
fn help() -> ! {
    let name_upper = nino_constants::PROGRAM_NAME.to_string().to_uppercase();
    eprintln!(
        "{} {}
        usage : nino postgres_connection_string
        or define {} environment valiable
        ex: export {}=postgres_connection_string",
        name_upper,
        nino_constants::PROGRAM_VERSION,
        name_upper,
        name_upper
    );
    std::process::exit(1);
}

fn setup_panic_hook() {
    use std::env;
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let name_upper = nino_constants::PROGRAM_NAME.to_string().to_uppercase();
        eprintln!(
            "!!! {} {} panic:",
            name_upper,
            nino_constants::PROGRAM_VERSION
        );
        eprintln!("Platform: {} {}", env::consts::OS, env::consts::ARCH);
        orig_hook(panic_info);
        std::process::exit(1);
    }));
}
