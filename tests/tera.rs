#![cfg(feature = "tera")]

use fluent_template_helper::*;
use tera::*;

simple_loader!(load, "./tests/locales", "en-US", core: "./tests/locales/core.ftl", customizer: |_bundle| {});

macro_rules! assert_templates {
    ($($test:ident ( $lang:expr ) => { $($content:expr => $assert:expr),+ }),+) => {
        $(
            #[test]
            fn $test() -> std::result::Result<(), Box<dyn std::error::Error>> {
                let ctx = Context::from_value(serde_json::json!({ "lang": $lang }))?;
                let mut tera = Tera::default();
                tera.register_function("fluent", FluentHelperFunction::new(load()));
                $(
                    tera.add_raw_template("_tmp.html", $content)?;
                    let render = tera.render("_tmp.html", &ctx)?;

                    assert_eq!($assert, render);
                )+
                    Ok(())
            }
        )+
    };
}

assert_templates! {
    english("en-US") => {
        r#"{{ fluent(_id="simple", _lang=lang) }}"# => "simple text",
        r#"{{ fluent(_id="reference", _lang=lang) }}"# => "simple text with a reference: foo",
        r#"{{ fluent(_id="parameter", _lang=lang, param="PARAM") }}"# => "text with a PARAM",
        r#"{{ fluent(_id="parameter2", _lang=lang, param1="P1", param2="P2") }}"# => "text one P1 second P2",
        r#"{{ fluent(_id="fallback", _lang=lang) }}"# => "this should fall back"
    },
    french("fr") => {
        r#"{{ fluent(_id="simple", _lang=lang) }}"# => "texte simple",
        r#"{{ fluent(_id="reference", _lang=lang) }}"# => "texte simple avec une référence: foo",
        r#"{{ fluent(_id="parameter", _lang=lang, param="PARAM") }}"# => "texte avec une PARAM",
        r#"{{ fluent(_id="parameter2", _lang=lang, param1="P1", param2="P2") }}"# => "texte une P1 seconde P2",
        r#"{{ fluent(_id="fallback", _lang=lang) }}"# => "this should fall back"
    },
    chinese("zh-TW") => {
        r#"{{ fluent(_id="exists", _lang=lang) }}"# => "兒",
        r#"{{ fluent(_id="fallback-zh", _lang=lang) }}"# => "气",
        r#"{{ fluent(_id="fallback", _lang=lang) }}"# => "this should fall back"
    }
}
