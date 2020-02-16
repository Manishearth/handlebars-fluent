#[cfg(feature = "handlebars")]
mod handlebars;
#[cfg(feature = "tera")]
mod tera;

/// The helper type that implements `handlebars::HelperDef` with the
/// `handlebars` feature and `tera::Function` with the `tera` feature.
#[allow(dead_code)]
pub struct FluentHelperFunction<L> {
    loader: L,
}

impl<L> FluentHelperFunction<L> {
    pub fn new(loader: L) -> Self {
        Self { loader }
    }
}
