mod db;
mod db_notification;
mod db_settings;
mod db_transactions;
mod js;
mod js_functions;
mod js_test;
mod js_test_debug;
mod nino_constants;
mod nino_functions;
mod nino_structures;
mod nino_trasport;
mod web;
mod web_dynamics;
mod web_requests;
mod web_statics;

use crate::{db_settings::SettingsManager, nino_constants::info};
use deno_core::anyhow::Error;
use nino_structures::InitialSettings;
use std::{fs, sync::Arc};

// export NINO=postgresql://george.radev@localhost/postgres?connect_timeout=5
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

async fn get_db_settings(connection_string: String) -> InitialSettings {
    let db;
    loop {
        match db::DBManager::instance(connection_string.clone(), 2).await {
            Ok(inst) => {
                db = Arc::new(inst);
                break;
            }
            Err(error) => eprintln!("Waiting for database...({})", error),
        };
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    info!("Database ok");

    let settings = SettingsManager::new(db, None);
    let thread_count = settings
        .get_setting_usize(
            nino_constants::SETTINGS_NINO_THREAD_COUNT,
            nino_constants::SETTINGS_NINO_THREAD_COUNT_DEFAULT,
        )
        .await;
    let js_thread_count = settings
        .get_setting_usize(
            nino_constants::SETTINGS_JS_THREAD_COUNT,
            nino_constants::SETTINGS_JS_THREAD_COUNT_DEFAULT,
        )
        .await;
    let debug_port = settings
        .get_setting_i32(
            nino_constants::SETTINGS_NINO_DEBUG_PORT,
            nino_constants::SETTINGS_NINO_DEBUG_PORT_DEFAULT,
        )
        .await as u16;
    let mut db_pool_size = settings
        .get_setting_usize(
            nino_constants::SETTINGS_DB_CONNECTION_POOL_SIZE,
            nino_constants::SETTINGS_DB_CONNECTION_POOL_SIZE_DEFAULT,
        )
        .await;
    if db_pool_size == 0 {
        // match db pool to serving threads + js threads
        db_pool_size = thread_count + js_thread_count;
    }

    InitialSettings {
        connection_string: connection_string.clone(),
        thread_count,
        debug_port,
        db_pool_size,
        js_thread_count,
    }
}

async fn main_async(settings: InitialSettings) {
    if let Err(error) = nino_init(settings).await {
        eprintln!("ERROR {}:{}:{}", file!(), line!(), error);
        std::process::exit(1);
    }
}
async fn nino_init(settings: InitialSettings) -> Result<(), Error> {
    let db = Arc::new(
        db::DBManager::instance(settings.connection_string.clone(), settings.db_pool_size).await?,
    );

    let db_notifier = db_notification::DBNotificationManager::new(db.clone());
    {
        // transport initial db content
        let transport = nino_trasport::TransportManager::new(db.clone());
        transport
            .transport_file("./transports/_transport.json")
            .await?;
    }

    let settings_manager = Arc::new(SettingsManager::new(
        db.clone(),
        Some(db_notifier.get_subscriber()),
    ));

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

    js::JavaScriptManager::create(
        settings.js_thread_count,
        settings.debug_port,
        db.get_connection_string(),
        dynamics.clone(),
        settings_manager.clone(),
    );
    {
        // compile dynamics
        let recompile_dynamics = fs::read_to_string("./transports/_recompile_dynamics.js")?;
        js::JavaScriptManager::run(&recompile_dynamics).await?;
    }

    let web = web::WebManager::new(
        settings_manager.clone(),
        requests.clone(),
        statics.clone(),
        dynamics.clone(),
    )
    .await;

    // brodcast initial message
    tokio::spawn(async move { notifier.notify("to all".to_string()).await });

    web.start().await?;
    Ok(())
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
