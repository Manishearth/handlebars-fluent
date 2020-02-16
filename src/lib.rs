//! [Fluent](https://projectfluent.org/) functions for templating engines.
//!
//! This crate provides a way to load Fluent strings from different
//! templating engines.
//! # Current Supported Template Engines
//! - [Handlebars](https://docs.rs/handlebars).
//! - [Tera](https://docs.rs/tera).
//!
//! # Set up
//! The first thing you need is a `Loader` to load and  render fluent templates.
//! The easiest way to do this currently is to use the [`simple_loader!()`]
//! macro. Then you need to define the `fluent` function with your templating
//! engine.  This is will vary slightly depending on your templating engine.
//!
//! ## Handlebars
//!
//! ```rust
//! # #[cfg(feature = "handlebars")] {
//! use fluent_template_helper::FluentHelperFunction;
//!
//! fluent_template_helper::simple_loader!(create_loader, "./locales/", "en-US");
//!
//! fn init(engine: &mut handlebars::Handlebars) {
//!     // Pick your engine
//!     let loader = create_loader();
//!     let helper = FluentHelperFunction::new(loader);
//!
//!     // TODO: Delete one of these lines
//!     // Handlebars
//!     engine.register_helper("fluent", Box::new(helper));
//! }
//!
//! fn render_page(engine: &handlebars::Handlebars) -> String {
//!     let data = serde_json::json!({"lang": "zh-CN"});
//!     // TODO: Delete one of these lines
//!     // Handlebars
//!     engine.render_template("{{fluent \"foo-bar\"}} baz", &data).unwrap()
//! }
//! # }
//! ```
//!
//! ## Tera
//!
//! ```rust
//! # #[cfg(feature = "tera")] {
//! use fluent_template_helper::FluentHelperFunction;
//!
//! fluent_template_helper::simple_loader!(create_loader, "./locales/", "en-US");
//!
//! fn init(engine: &mut tera::Tera) {
//!     // Pick your engine
//!     let loader = create_loader();
//!     let helper = FluentHelperFunction::new(loader);
//!
//!     engine.register_function("fluent", helper);
//!     engine.add_raw_template("foo", "{{fluent \"foo-bar\"}} baz").unwrap();
//! }
//!
//! fn render_page(engine: &tera::Tera) -> String {
//!     let data = serde_json::json!({"lang": "zh-CN"});
//!     let context = tera::Context::from_value(data).unwrap();
//!     engine.render("foo", &context).unwrap()
//! }
//! # }
//! ```
//!
//! You should have a `locales/` folder somewhere with one folder per language code,
//! containing all of your FTL files. See the [`simple_loader!()`] macro for more options.
//!
//! # Handlebars Syntax
//!
//! Make sure the [`handlebars::Context`] has a toplevel "lang" field when rendering.
//!
//! The main helper provided is the `{{fluent}}` helper. If you have the following Fluent
//! file:
//!
//! ```fluent
//! foo-bar = "foo bar"
//! placeholder = this has a placeholder { $variable }
//! ```
//!
//! You can include the strings in your template with
//!
//! ```hbs
//! {{fluent "foo-bar"}} <!-- will render "foo bar" -->
//! {{fluent "placeholder" variable="baz"}} <!-- will render "this has a placeholder baz" -->
//!```
//!
//! You may also use the `{{fluentparam}}` helper to specify [variables], especially if you need
//! them to be multiline, like so:
//!
//! ```hbs
//! {{#fluent "placeholder"}}
//!     {{#fluentparam "variable"}}
//!         first line
//!         second line
//!     {{/fluentparam}}
//! {{/fluent}}
//! ```
//!
//! Multiple `{{fluentparam}}`s may be specified
//!
//! [variables]: https://projectfluent.org/fluent/guide/variables.html
//! [`simple_loader!()`]: ./macro.simple_loader.html
//!
//! # Tera Syntax
//! For `Tera` we provide a `tera::Function` implementation. You need provide
//! both a `_id` pointing to the fluent property and `_lang` pointing to a
//! language code. Any extra parameters will be sent as variables to fluent.
//!
//! ## Basic
//! ```tera
//! {{ fluent(_fluent_key="foo-bar", _lang="en-US") }}
//! ```
//!
//! ## With parameters
//! ```tera
//! {{ fluent(_fluent_key="foo-bar", _lang="en-US", param="value") }}
//! ```
//!
//! Instead of `fluentparam` in handlebars you can [manipulate data][mand]
//! directly with Tera templates.
//!
//! ```tera
//! {% set link = '<a href="' ~ url ~ '">' ~ url_text ~ '</a>' %}
//! {{ fluent(_fluent_key="foo-bar", _lang="en-US", param=link) }}
//! ```
//!
//! [mand]: https://tera.netlify.com/docs/#manipulating-data
//!

#[doc(hidden)]
pub extern crate lazy_static;

#[doc(hidden)]
pub extern crate fluent_bundle;

pub use helper::FluentHelperFunction;
pub use loader::{Loader, SimpleLoader};

mod helper;
pub mod loader;
