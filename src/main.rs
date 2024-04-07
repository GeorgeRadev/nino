mod db;
mod db_notification;
mod db_settings;
mod db_transactions;
mod js;
mod js_functions;
mod js_test;
mod js_test_debug;
mod js_worker;
mod nino_constants;
mod nino_functions;
mod nino_structures;
mod web;
mod web_requests;
mod web_responses;

use crate::{db_settings::SettingsManager, nino_constants::info};
use deno_runtime::deno_core::anyhow::{anyhow, Error};
use nino_structures::InitialSettings;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;

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
            .block_on(main_init(connection_string))
    };

    // async functionalities goes here
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(initial_settings.thread_count)
        .enable_all()
        .build()
        .unwrap()
        .block_on(main_async(initial_settings));
}

async fn main_init(connection_string: String) -> InitialSettings {
    // wait for DB availability
    wait_for_db_connection(connection_string.clone()).await;
    // check if there is a .sql file available and execute it in the DB
    // used for initial DB setup and migration purposes
    execute_migration_sql_if_needed(connection_string.clone())
        .await
        .unwrap();
    // get db settings
    get_db_settings(connection_string).await
}

async fn wait_for_db_connection(connection_string: String) {
    loop {
        match db::DBManager::instance(connection_string.clone(), 1).await {
            Ok(_) => {
                break;
            }
            Err(error) => eprintln!("Waiting for database...({})", error),
        };
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    info!("Database ok");
}

async fn execute_migration_sql_if_needed(connection_string: String) -> Result<(), Error> {
    let db = db::DBManager::instance(connection_string.clone(), 1)
        .await
        .unwrap();

    let file = match tokio::fs::File::open(format!("{}.sql", nino_constants::PROGRAM_NAME)).await {
        Ok(file) => file,
        Err(_) => {
            // error opening the init sql script
            // just continue
            return Ok(());
        }
    };
    let reader = tokio::io::BufReader::new(file);
    let mut lines = reader.lines();

    let mut sql = String::new();
    while let Some(line) = lines.next_line().await? {
        if line.ends_with(";") {
            // execute
            sql.push_str(&line);
            if let Err(error) = db.execute(&sql, &[]).await {
                return Err(anyhow!("sql:{}\nerror: {}", sql, error.to_string()));
            }
            sql.clear();
        } else {
            sql.push_str(&line);
            sql.push('\n');
        }
    }
    Ok(())
}

async fn get_db_settings(connection_string: String) -> InitialSettings {
    let db = Arc::new(
        db::DBManager::instance(connection_string.clone(), 1)
            .await
            .unwrap(),
    );
    let settings = SettingsManager::new(db, None);

    let system_id = settings
        .get_setting_str(
            nino_constants::SETTINGS_NINO_SYSTEM_ID,
            nino_constants::SETTINGS_NINO_SYSTEM_ID_DEFAULT,
        )
        .await;
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
        system_id,
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

    let settings_manager = Arc::new(SettingsManager::new(
        db.clone(),
        Some(db_notifier.get_subscriber()),
    ));

    let requests = Arc::new(web_requests::RequestManager::new(
        db.clone(),
        db_notifier.get_subscriber(),
    ));

    let dyn_subscriber = db_notifier.get_subscriber();
    let notifier = Arc::new(db_notification::Notifier::new(Arc::new(db_notifier)));

    let responses = Arc::new(web_responses::ResponseManager::new(
        db.clone(),
        settings.js_thread_count,
        notifier.clone(),
        dyn_subscriber,
    ));

    js::JavaScriptManager::create(
        settings.js_thread_count,
        settings.debug_port,
        db.get_connection_string(),
        responses.clone(),
        settings_manager.clone(),
    );

    // transpile dynamics
    if let Ok(transpile_code) = responses
        .get_response_bytes(crate::nino_constants::TRANSPILE_MODULE)
        .await
    {
        let transpile_code = String::from_utf8(transpile_code)?;
        js::JavaScriptManager::run(&transpile_code).await?;
    }

    let web = web::WebManager::new(
        settings_manager.clone(),
        requests.clone(),
        responses.clone(),
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
