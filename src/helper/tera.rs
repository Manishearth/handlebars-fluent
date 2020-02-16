use fluent_bundle::FluentValue;
use serde_json::Value as Json;
use snafu::OptionExt;
use std::collections::HashMap;

use super::FluentHelperFunction;
use crate::Loader;

type Result<T, E = Error> = std::result::Result<T, E>;

const LANG_KEY: &str = "_lang";
const FLUENT_KEY: &str = "_id";

#[derive(Debug, snafu::Snafu)]
enum Error {
    #[snafu(display("No `lang` argument provided."))]
    NoLangArgument,
    #[snafu(display("No `_id` argument provided."))]
    NoFluentArgument,
    #[snafu(display("Couldn't convert JSON to Fluent value."))]
    JsonToFluentFail,
}

impl From<Error> for tera::Error {
    fn from(error: Error) -> Self {
        tera::Error::msg(error)
    }
}

fn json_to_fluent(json: Json) -> Result<FluentValue> {
    match json {
        Json::Number(ref n) => Ok(FluentValue::Number(n.to_string())),
        Json::String(ref s) => Ok(FluentValue::String(s.to_string())),
        _ => Err(Error::JsonToFluentFail),
    }
}

impl<L: Loader + Send + Sync> tera::Function for FluentHelperFunction<L> {
    fn call(&self, args: &HashMap<String, Json>) -> Result<Json, tera::Error> {
        let lang = args
            .get(LANG_KEY)
            .and_then(Json::as_str)
            .context(self::NoLangArgument)?;
        let id = args
            .get(FLUENT_KEY)
            .and_then(Json::as_str)
            .context(self::NoFluentArgument)?;
        let fluent_values = args
            .values()
            .cloned()
            .map(json_to_fluent)
            .collect::<Result<Vec<_>>>()?;

        let fluent_args = args.keys().map(String::as_str).zip(fluent_values).collect();

        let response = self.loader.lookup(lang, &id, Some(&fluent_args));

        Ok(Json::String(response))
    }
}
