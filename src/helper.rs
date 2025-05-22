use handlebars::{
    Context, Handlebars, Helper, HelperDef, HelperResult, Output, RenderContext, RenderErrorReason,
    Renderable,
};

use fluent_bundle::{FluentArgs, FluentValue};
use handlebars::template::{Parameter, TemplateElement};
use serde_json::Value as Json;
use std::io;

use crate::Loader;

pub struct FluentHelper<L> {
    loader: L,
}

impl<L> FluentHelper<L> {
    pub fn new(loader: L) -> Self {
        Self { loader }
    }
}

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

impl<L: Loader + Send + Sync> HelperDef for FluentHelper<L> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'rc>,
        reg: &'reg Handlebars,
        context: &'rc Context,
        rcx: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let id = if let Some(id) = h.param(0) {
            id
        } else {
            return Err(RenderErrorReason::ParamNotFoundForIndex("fluent", 0).into());
        };

        if id.relative_path().is_some() {
            return Err(RenderErrorReason::ParamTypeMismatchForName(
                "fluent",
                "0".into(),
                "string with no path".into(),
            )
            .into());
        }

        let id = if let Json::String(ref s) = *id.value() {
            s
        } else {
            return Err(RenderErrorReason::ParamTypeMismatchForName(
                "fluent",
                "0".into(),
                "string".into(),
            )
            .into());
        };

        let mut args = if h.hash().is_empty() {
            None
        } else {
            let mut fluent_args = FluentArgs::new();
            for (k, v) in h.hash() {
                let k = k.to_owned();
                match v.value() {
                    // `Number::as_f64` can't fail here because we haven't
                    // enabled `arbitrary_precision` feature in `serde_json`.
                    Json::Number(n) => fluent_args.set(k, n.as_f64().unwrap()),
                    Json::String(s) => fluent_args.set(k, s.to_owned()),
                    _ => continue,
                }
            }
            Some(fluent_args)
        };

        if let Some(tpl) = h.template() {
            if args.is_none() {
                args = Some(FluentArgs::new());
            }
            let args = args.as_mut().unwrap();
            for element in &tpl.elements {
                if let TemplateElement::HelperBlock(ref block) = element {
                    if block.name != Parameter::Name("fluentparam".into()) {
                        return Err(RenderErrorReason::Other(format!(
                            "{{{{fluent}}}} can only contain {{{{fluentparam}}}} elements, not {}",
                            block.name.expand_as_name(reg, context, rcx).unwrap()
                        ))
                        .into());
                    }
                    let id = if let Some(el) = block.params.first() {
                        if let Parameter::Literal(Json::String(ref s)) = *el {
                            s
                        } else {
                            return Err(RenderErrorReason::ParamTypeMismatchForName(
                                "fluentparam",
                                "0".into(),
                                "string".into(),
                            )
                            .into());
                        }
                    } else {
                        return Err(
                            RenderErrorReason::ParamNotFoundForIndex("fluentparam", 0).into()
                        );
                    };
                    if let Some(ref tpl) = block.template {
                        let mut s = StringOutput::default();
                        tpl.render(reg, context, rcx, &mut s)?;
                        args.set(id, FluentValue::String(s.s.into()));
                    }
                }
            }
        }
        let lang = context
            .data()
            .get("lang")
            .expect("Language not set in context")
            .as_str()
            .expect("Language must be string")
            .parse()
            .expect("Language not valid identifier");

        let response = self.loader.lookup(&lang, id, args.as_ref());
        out.write(&response)
            .map_err(|e| RenderErrorReason::NestedError(Box::new(e)).into())
    }
}
