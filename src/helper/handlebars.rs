use handlebars::{
    Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext, RenderError,
    Renderable,
};

use fluent_bundle::FluentValue;
use handlebars::template::{Parameter, TemplateElement};
use serde_json::Value as Json;
use std::collections::HashMap;
use std::io;

use super::FluentHelperFunction;
use crate::Loader;

#[derive(Default)]
struct StringOutput {
    pub s: String,
}

impl Output for StringOutput {
    fn write(&mut self, seg: &str) -> Result<(), io::Error> {
        self.s.push_str(seg);
        Ok(())
    }
}

impl<L: Loader + Send + Sync> HelperDef for FluentHelperFunction<L> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        reg: &'reg Handlebars,
        context: &'rc Context,
        rcx: &mut RenderContext<'reg>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let id = if let Some(id) = h.param(0) {
            id
        } else {
            return Err(RenderError::new(
                "{{fluent}} must have at least one parameter",
            ));
        };

        if id.path().is_some() {
            return Err(RenderError::new(
                "{{fluent}} takes a string parameter with no path",
            ));
        }

        let id = if let Json::String(ref s) = *id.value() {
            s
        } else {
            return Err(RenderError::new("{{fluent}} takes a string parameter"));
        };

        let mut args = if h.hash().is_empty() {
            None
        } else {
            let map = h
                .hash()
                .iter()
                .filter_map(|(k, v)| {
                    let json = v.value();
                    let val = match *json {
                        Json::Number(ref n) => FluentValue::Number(n.to_string()),
                        Json::String(ref s) => FluentValue::String(s.to_string()),
                        _ => return None,
                    };
                    Some((&**k, val))
                })
                .collect();
            Some(map)
        };

        if let Some(tpl) = h.template() {
            if args.is_none() {
                args = Some(HashMap::new());
            }
            let args = args.as_mut().unwrap();
            for element in &tpl.elements {
                if let TemplateElement::HelperBlock(ref block) = element {
                    if block.name != Parameter::Name("fluentparam".into()) {
                        return Err(RenderError::new(format!(
                            "{{{{fluent}}}} can only contain {{{{fluentparam}}}} elements, not {}",
                            block.name.expand_as_name(reg, context, rcx).unwrap()
                        )));
                    }
                    let id = if let Some(el) = block.params.get(0) {
                        if let Parameter::Literal(ref s) = *el {
                            if let Json::String(ref s) = *s {
                                s
                            } else {
                                return Err(RenderError::new(
                                    "{{fluentparam}} takes a string parameter",
                                ));
                            }
                        } else {
                            return Err(RenderError::new(
                                "{{fluentparam}} takes a string parameter",
                            ));
                        }
                    } else {
                        return Err(RenderError::new("{{fluentparam}} must have one parameter"));
                    };
                    if let Some(ref tpl) = block.template {
                        let mut s = StringOutput::default();
                        tpl.render(reg, context, rcx, &mut s)?;
                        args.insert(&*id, FluentValue::String(s.s.into()));
                    }
                }
            }
        }
        let lang = context
            .data()
            .get("lang")
            .expect("Language not set in context")
            .as_str()
            .expect("Language must be string");

        let response = self.loader.lookup(lang, &id, args.as_ref());
        out.write(&response).map_err(RenderError::with)
    }
}
