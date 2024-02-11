use crate::{db::DBManager, nino_constants};
use deno_runtime::deno_core::{
    anyhow::Error,
    serde_json::{self, Map, Value},
};
use std::{collections::HashMap, fs, io::Read, path::Path, sync::Arc};

pub struct TransportManager {
    db: Arc<DBManager>,
}

impl TransportManager {
    pub fn new(db: Arc<DBManager>) -> TransportManager {
        TransportManager { db }
    }

    pub async fn transport_file(&self, file_name: &str) -> Result<(), Error> {
        let file_content = fs::read_to_string(file_name)?;
        let transport_path = Path::new(file_name)
            .parent()
            .unwrap()
            .as_os_str()
            .to_str()
            .unwrap();
        let object: HashMap<String, Value> = serde_json::from_str(&file_content)?;
        self.transport_object(&object, transport_path).await?;
        Ok(())
    }

    pub async fn transport_object<'t>(
        &self,
        transport: &HashMap<String, Value>,
        transport_path: &str,
    ) -> Result<(), Error> {
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

    async fn transport_queries(&self, queries: Option<&Vec<Value>>) -> Result<(), Error> {
        if queries.is_none() {
            return Ok(());
        }
        let queries = queries.unwrap();
        for (ix, query) in queries.iter().enumerate() {
            let obj_name = "queries";
            let query = Self::get_object(obj_name, query, ix);

            let query_string = Self::get_string(obj_name, query, ix, "query");
            let break_on_error = Self::get_bool(query, "break_on_error", true);

            if let Err(error) = self.db.execute(query_string, &[]).await {
                if break_on_error {
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    async fn transport_settings(&self, settings: Option<&Vec<Value>>) -> Result<(), Error> {
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
            self.db
                .execute(&query, &[&key_string, &value_string])
                .await?;
        }
        Ok(())
    }

    async fn transport_requests(&self, requests: Option<&Vec<Value>>) -> Result<(), Error> {
        if requests.is_none() {
            return Ok(());
        }
        let requests = requests.unwrap();
        for (ix, request) in requests.iter().enumerate() {
            let obj_name = "requests";
            let request = Self::get_object(obj_name, request, ix);

            let path = Self::get_string(obj_name, request, ix, "path");
            let name = Self::get_string(obj_name, request, ix, "name");
            let mime = Self::get_string(obj_name, request, ix, "mime");
            let redirect = Self::get_bool(request, "redirect", false);
            let authorize = Self::get_bool(request, "authorize", false);
            let dynamic = Self::get_bool(request, "dynamic", false);
            let execute = Self::get_bool(request, "execute", false);

            let query = format!(
                "INSERT INTO {} (path, name, mime, redirect, authorize, dynamic, execute) VALUES($1, $2, $3, $4, $5, $6, $7)",
                nino_constants::REQUESTS_TABLE
            );
            self.db
                .execute(
                    &query,
                    &[
                        &path, &name, &mime, &redirect, &authorize, &dynamic, &execute,
                    ],
                )
                .await?;
        }
        Ok(())
    }

    async fn transport_statics(
        &self,
        statics: Option<&Vec<Value>>,
        transport_path: &str,
    ) -> Result<(), Error> {
        if statics.is_none() {
            return Ok(());
        }
        let statics = statics.unwrap();
        for (ix, statics) in statics.iter().enumerate() {
            let obj_name = "statics";
            let statics = Self::get_object(obj_name, statics, ix);

            let name = Self::get_string(obj_name, statics, ix, "name");
            let file_name = Self::get_string(obj_name, statics, ix, "file");
            let (length, content) = Self::get_file(transport_path, file_name)?;

            let query = format!(
                "INSERT INTO {}(name, length, content) VALUES($1, $2, $3)",
                nino_constants::STATICS_TABLE
            );
            self.db.execute(&query, &[&name, &length, &content]).await?;
        }
        Ok(())
    }

    async fn transport_dynamics(
        &self,
        dynamics: Option<&Vec<Value>>,
        transport_path: &str,
    ) -> Result<(), Error> {
        if dynamics.is_none() {
            return Ok(());
        }
        let dynamics = dynamics.unwrap();
        for (ix, dynamic) in dynamics.iter().enumerate() {
            let obj_name = "dynamics";
            let dynamic = Self::get_object(obj_name, dynamic, ix);

            let name_string = Self::get_string(obj_name, dynamic, ix, "name");
            let file_name = Self::get_string(obj_name, dynamic, ix, "file");
            let transpile = Self::get_bool(dynamic, "transpile", false);

            let (length, content) = Self::get_file(transport_path, file_name)?;

            let query = format!(
                "INSERT INTO {}(name, code_length, js_length, transpile, code, js) VALUES($1, $2, $2, $4, $3, $3)",
                nino_constants::DYNAMICS_TABLE
            );
            self.db
                .execute(&query, &[&name_string, &length, &content, &transpile])
                .await?;
        }
        Ok(())
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
        Self::get_object_string(obj, field_name).unwrap_or_else(|_| {
            panic!(
                "'{}'[{}] does not contain string value for key '{}'",
                obj_name, ix, field_name
            )
        })
    }

    fn get_bool(obj: &Map<String, Value>, field_name: &str, default: bool) -> bool {
        Self::get_object_boolean(obj, field_name, default)
    }

    fn get_file(transport_path: &str, file_name: &str) -> Result<(i32, Vec<u8>), Error> {
        let file = Path::new(transport_path).join(file_name);
        let mut fd = std::fs::File::open(file)?;
        let mut content = Vec::new();
        fd.read_to_end(&mut content)?;
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
