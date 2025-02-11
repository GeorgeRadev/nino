use crate::db_settings::SettingsManager;
use crate::web_responses::ResponseManager;
use crate::{js_core, js_functions, nino_constants};
use deno_core::{anyhow::Error, url::Url};
use std::sync::Arc;

/// need to call start() to begin js threads
pub struct JavaScriptManager {}

impl JavaScriptManager {
    /**
     * Create and initialize js threads.
     */
    pub fn create(
        thread_count: usize,
        inspector_port: u16,
        connection_string: String,
        dynamics: Arc<ResponseManager>,
        settings: Arc<SettingsManager>,
    ) -> Result<(), Error> {
        js_core::js_init(js_functions::load_module, 4);
        let main_uri = format!("{}{}", js_core::MODULE_URI, nino_constants::MODULE_MAIN).to_owned();
        let main_module = Url::parse(main_uri.as_str())?;

        js_functions::init_js_context(connection_string, dynamics, settings);
        for id in 0..thread_count {
            js_core::start_js_thread(
                js_functions::nino_extentions,
                js_core::ExecuteMode::JsModule(main_module.clone()),
                true,
                if id == 0 { inspector_port } else { 0 },
            )?;
        }

        Ok(())
    }

    pub fn run(code: String) -> Result<(), Error> {
        js_core::start_js_thread(
            js_functions::nino_extentions,
            js_core::ExecuteMode::JsCode(code.clone()),
            false,
            0,
        )?;

        Ok(())
    }
}
