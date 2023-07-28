use deno_core::serde_json::{self, Map, Value};

use crate::{db::DBManager, nino_constants};
use postgres::types::ToSql;
use std::{collections::HashMap, fs, io::Read, path::Path, sync::Arc};

pub struct TransportManager {
    db: Arc<DBManager>,
}

impl TransportManager {
    pub fn new(db: Arc<DBManager>) -> TransportManager {
        TransportManager { db }
    }

    pub async fn transport_file(&self, file_name: &str) -> Result<(), String> {
        let file_content = fs::read_to_string(file_name).map_err(|e| e.to_string())?;
        let transport_path = Path::new(file_name)
            .parent()
            .unwrap()
            .as_os_str()
            .to_str()
            .unwrap();
        let object: HashMap<String, Value> =
            serde_json::from_str(&file_content).map_err(|e| e.to_string())?;
        self.transport_object(&object, transport_path).await?;
        Ok(())
    }

    pub async fn transport_object<'t>(
        &self,
        transport: &HashMap<String, Value>,
        transport_path: &str,
    ) -> Result<(), String> {
        self.transport_queries(Self::get_object_array(transport, "queries"))
            .await?;
        self.transport_settings(Self::get_object_array(transport, "settings"))
            .await?;
        self.transport_requests(Self::get_object_array(transport, "requests"))
            .await?;
        self.transport_statics(Self::get_object_array(transport, "statics"), transport_path)
            .await?;
        self.transport_dynamics(
            Self::get_object_array(transport, "dynamics"),
            transport_path,
        )
        .await?;
        Ok(())
    }

    async fn transport_queries(&self, queries: Option<&Vec<Value>>) -> Result<(), String> {
        if queries.is_none() {
            return Ok(());
        }
        let queries = queries.unwrap();
        for (ix, query) in queries.iter().enumerate() {
            let obj_name = "queries";
            let query = Self::get_object(obj_name, query, ix);

            let query_string = Self::get_string(obj_name, query, ix, "query");
            let break_on_error = Self::get_bool(query, "break_on_error", true);

            let result = self.db.execute(query_string, &[]).await;
            if result.is_err() {
                let log_level = if break_on_error { "ERROR" } else { "WARNING" };
                let err_str = format!(
                    "{}:{}:{}:{}",
                    log_level,
                    file!(),
                    line!(),
                    result.err().unwrap()
                );
                eprintln!("{}", err_str);
                if break_on_error {
                    return Err(err_str);
                } 
            }
        }
        Ok(())
    }

    async fn transport_settings(&self, settings: Option<&Vec<Value>>) -> Result<(), String> {
        if settings.is_none() {
            return Ok(());
        }
        let settings = settings.unwrap();
        for (ix, setting) in settings.iter().enumerate() {
            let obj_name = "settings";
            let setting = Self::get_object(obj_name, setting, ix);

            let key_string = Self::get_string(obj_name, setting, ix, "key");
            let value_string = Self::get_string(obj_name, setting, ix, "value");

            let query = format!(
                "INSERT INTO {} (setting_key, setting_value) VALUES ($1, $2)",
                nino_constants::SETTINGS_TABLE
            );
            Self::execute_query(&self.db, query, &[&key_string, &value_string]).await?;
        }
        Ok(())
    }

    async fn transport_requests(&self, requests: Option<&Vec<Value>>) -> Result<(), String> {
        if requests.is_none() {
            return Ok(());
        }
        let requests = requests.unwrap();
        for (ix, request) in requests.iter().enumerate() {
            let obj_name = "requests";
            let request = Self::get_object(obj_name, request, ix);

            let path_string = Self::get_string(obj_name, request, ix, "path");
            let name_string = Self::get_string(obj_name, request, ix, "name");
            let dynamic_bool = Self::get_bool(request, "dynamic", false);
            let execute_bool = Self::get_bool(request, "execute", false);
            let authorize_bool = Self::get_bool(request, "authorize", false);

            let query = format!(
                "INSERT INTO {}(path, name, dynamic, execute, authorize) VALUES($1, $2, $3, $4, $5)",
                nino_constants::REQUESTS_TABLE
            );
            Self::execute_query(
                &self.db,
                query,
                &[&path_string, &name_string, &dynamic_bool, &execute_bool, &authorize_bool],
            )
            .await?;
        }
        Ok(())
    }

    async fn transport_statics(
        &self,
        statics: Option<&Vec<Value>>,
        transport_path: &str,
    ) -> Result<(), String> {
        if statics.is_none() {
            return Ok(());
        }
        let statics = statics.unwrap();
        for (ix, statics) in statics.iter().enumerate() {
            let obj_name = "statics";
            let statics = Self::get_object(obj_name, statics, ix);

            let name_string = Self::get_string(obj_name, statics, ix, "name");
            let mime_string = Self::get_string(obj_name, statics, ix, "mime");
            let file_name = Self::get_string(obj_name, statics, ix, "file");
            let (length, content) = Self::get_file(transport_path, file_name)?;

            let query = format!(
                "INSERT INTO {}(name, mime, length, content) VALUES($1, $2, $3, $4)",
                nino_constants::STATICS_TABLE
            );
            Self::execute_query(
                &self.db,
                query,
                &[&name_string, &mime_string, &length, &content],
            )
            .await?;
        }
        Ok(())
    }

    async fn transport_dynamics(
        &self,
        dynamics: Option<&Vec<Value>>,
        transport_path: &str,
    ) -> Result<(), String> {
        if dynamics.is_none() {
            return Ok(());
        }
        let dynamics = dynamics.unwrap();
        for (ix, dynamic) in dynamics.iter().enumerate() {
            let obj_name = "dynamics";
            let dynamic = Self::get_object(obj_name, dynamic, ix);

            let name_string = Self::get_string(obj_name, dynamic, ix, "name");
            let file_name = Self::get_string(obj_name, dynamic, ix, "file");
            let (length, content) = Self::get_file(transport_path, file_name)?;

            let query = format!(
                "INSERT INTO {}(name, code_length, js_length, code, js) VALUES($1, $2, $2, $3, $3)",
                nino_constants::DYNAMICS_TABLE
            );
            Self::execute_query(&self.db, query, &[&name_string, &length, &content]).await?;
        }
        Ok(())
    }

    async fn execute_query(
        db: &DBManager,
        query: String,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<(), String> {
        let result = db.execute(query.as_str(), params).await;
        if result.is_err() {
            let err_str = format!(
                "ERROR:{}:{}:{} after query: {}",
                file!(),
                line!(),
                result.err().unwrap(),
                query
            );
            eprintln!("{}", err_str);
            Err(err_str)
        } else {
            Ok(())
        }
    }

    fn get_object<'a>(obj_name: &'a str, obj: &'a Value, ix: usize) -> &'a Map<String, Value> {
        obj.as_object()
            .unwrap_or_else(|| panic!("'{}'[{}] is not an object", obj_name, ix))
    }

    fn get_string<'a>(
        obj_name: &'a str,
        obj: &'a Map<String, Value>,
        ix: usize,
        field_name: &'a str,
    ) -> &'a str {
        Self::get_object_string(obj, field_name).unwrap_or_else(|_| panic!("'{}'[{}] does not contain string value for key '{}'",
            obj_name, ix, field_name))
    }

    fn get_bool(obj: &Map<String, Value>, field_name: &str, default: bool) -> bool {
        Self::get_object_boolean(obj, field_name, default)
    }

    fn get_file(transport_path: &str, file_name: &str) -> Result<(i32, Vec<u8>), String> {
        let file = Path::new(transport_path).join(file_name);
        let mut fd = std::fs::File::open(file).map_err(|e| e.to_string())?;
        let mut content = Vec::new();
        fd.read_to_end(&mut content).map_err(|e| e.to_string())?;
        let length = content.len() as i32;
        Ok((length, content))
    }

    fn get_object_string<'b>(v: &'b Map<String, Value>, key: &str) -> Result<&'b str, ()> {
        if v.contains_key(key) {
            let v = v.get(key).unwrap();
            if v.is_string() {
                return Ok(v.as_str().unwrap());
            }
        }
        Err(())
    }

    fn get_object_array<'b>(v: &'b HashMap<String, Value>, key: &str) -> Option<&'b Vec<Value>> {
        if v.contains_key(key) {
            let v = v.get(key).unwrap();
            if v.is_array() {
                return Some(v.as_array().unwrap());
            }
        }
        None
    }

    fn get_object_boolean(object: &Map<String, Value>, key: &str, default: bool) -> bool {
        if object.contains_key(key) {
            if let Some(v) = object.get(key) {
                if v.is_boolean() {
                    return v.as_bool().unwrap();
                }
            }
        }
        default
    }
}
