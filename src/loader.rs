use std::collections::HashMap;
use std::fs::read_dir;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentResource, FluentValue};
use fluent_langneg::negotiate_languages;

pub use unic_langid::{langid, langids, LanguageIdentifier};

/// Something capable of looking up Fluent keys given a language.
///
/// Use [SimpleLoader] if you just need the basics
pub trait Loader {
    fn lookup(
        &self,
        lang: &LanguageIdentifier,
        text_id: &str,
        args: Option<&HashMap<&str, FluentValue>>,
    ) -> String;
}

/// Loads Fluent data at runtime via `lazy_static` to produce a loader.
///
/// Usage:
///
/// ```rust
/// use handlebars_fluent::*;
///
/// simple_loader!(create_loader, "./tests/locales/", "en-US");
///
/// fn init() {
///     let loader = create_loader();
///     let helper = FluentHelper::new(loader);
/// }
/// ```
///
/// `$constructor` is the name of the constructor function for the loader, `$location` is
/// the location of a folder containing individual locale folders, `$fallback` is the language to use
/// for fallback strings.
///
/// Some Fluent users have a share "core.ftl" file that contains strings used by all locales,
/// for example branding information. They also may want to define custom functions on the bundle.
///
/// This can be done with an extended invocation:
///
/// ```rust
/// use handlebars_fluent::*;
///
/// simple_loader!(create_loader, "./tests/locales/", "en-US", core: "./tests/core.ftl",
///                customizer: |bundle| {bundle.add_function("FOOBAR", |_values, _named| {unimplemented!()}); });
///
/// fn init() {
///     let loader = create_loader();
///     let helper = FluentHelper::new(loader);
/// }
/// ```
///
/// The constructor function is cheap to call multiple times since all the heavy duty stuff is stored in shared statics.
///
#[macro_export]
macro_rules! simple_loader {
    ($constructor:ident, $location:expr, $fallback:expr) => {
        $crate::lazy_static::lazy_static! {
            static ref RESOURCES: std::collections::HashMap<$crate::loader::LanguageIdentifier, Vec<$crate::fluent_bundle::FluentResource>> = $crate::loader::build_resources($location);
            static ref BUNDLES: std::collections::HashMap<$crate::loader::LanguageIdentifier, $crate::fluent_bundle::concurrent::FluentBundle<&'static $crate::fluent_bundle::FluentResource>> = $crate::loader::build_bundles(&&RESOURCES, None, |_bundle| {});
            static ref LOCALES: Vec<$crate::loader::LanguageIdentifier> = RESOURCES.keys().cloned().collect();
            static ref FALLBACKS: std::collections::HashMap<$crate::loader::LanguageIdentifier, Vec<$crate::loader::LanguageIdentifier>> = $crate::loader::build_fallbacks(&*LOCALES);
        }

        pub fn $constructor() -> $crate::loader::SimpleLoader {
            $crate::loader::SimpleLoader::new(&*BUNDLES, &*FALLBACKS, $fallback.parse().expect("fallback language not valid"))
        }
    };
    ($constructor:ident, $location:expr, $fallback:expr, core: $core:expr, customizer: $custom:expr) => {
        $crate::lazy_static::lazy_static! {
            static ref CORE_RESOURCE: $crate::fluent_bundle::FluentResource = $crate::loader::load_core_resource($core);
            static ref RESOURCES: std::collections::HashMap<$crate::loader::LanguageIdentifier, Vec<$crate::fluent_bundle::FluentResource>> = $crate::loader::build_resources($location);
            static ref BUNDLES: std::collections::HashMap<$crate::loader::LanguageIdentifier, $crate::fluent_bundle::concurrent::FluentBundle<&'static $crate::fluent_bundle::FluentResource>> = $crate::loader::build_bundles(&*RESOURCES, Some(&CORE_RESOURCE), $custom);
            static ref LOCALES: Vec<$crate::loader::LanguageIdentifier> = RESOURCES.keys().cloned().collect();
            static ref FALLBACKS: std::collections::HashMap<$crate::loader::LanguageIdentifier, Vec<$crate::loader::LanguageIdentifier>> = $crate::loader::build_fallbacks(&*LOCALES);
        }

        pub fn $constructor() -> $crate::loader::SimpleLoader {
            $crate::loader::SimpleLoader::new(&*BUNDLES, &*FALLBACKS, $fallback.parse().expect("fallback language not valid"))
        }
    };
}

pub fn build_fallbacks(
    locales: &[LanguageIdentifier],
) -> HashMap<LanguageIdentifier, Vec<LanguageIdentifier>> {
    let mut map = HashMap::new();

    for locale in locales.iter() {
        map.insert(
            locale.to_owned(),
            negotiate_languages(
                &[locale],
                locales,
                None,
                fluent_langneg::NegotiationStrategy::Filtering,
            )
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
        );
    }

    map
}

/// A simple Loader implementation, with statically-loaded fluent data.
/// Typically created with the [`simple_loader!()`] macro
pub struct SimpleLoader {
    bundles: &'static HashMap<LanguageIdentifier, FluentBundle<&'static FluentResource>>,
    fallbacks: &'static HashMap<LanguageIdentifier, Vec<LanguageIdentifier>>,
    fallback: LanguageIdentifier,
}

impl SimpleLoader {
    /// Construct a SimpleLoader
    ///
    /// You should probably be using the constructor from `simple_loader!()`
    pub fn new(
        bundles: &'static HashMap<LanguageIdentifier, FluentBundle<&'static FluentResource>>,
        fallbacks: &'static HashMap<LanguageIdentifier, Vec<LanguageIdentifier>>,
        fallback: LanguageIdentifier,
    ) -> Self {
        Self {
            bundles,
            fallbacks,
            fallback,
        }
    }

    /// Convenience function to look up a string for a single language
    pub fn lookup_single_language(
        &self,
        lang: &LanguageIdentifier,
        text_id: &str,
        args: Option<&HashMap<&str, FluentValue>>,
    ) -> Option<String> {
        if let Some(bundle) = self.bundles.get(lang) {
            if let Some(message) = bundle.get_message(text_id).and_then(|m| m.value) {
                let mut errors = Vec::new();

                let value = bundle.format_pattern(message, dbg!(args), &mut errors);

                if errors.is_empty() {
                    Some(value.into())
                } else {
                    panic!(
                        "Failed to format a message for locale {} and id {}.\nErrors\n{:?}",
                        lang, text_id, errors
                    )
                }
            } else {
                None
            }
        } else {
            panic!("Unknown language {}", lang)
        }
    }

    /// Convenience function to look up a string without falling back to the default fallback language
    pub fn lookup_no_default_fallback(
        &self,
        lang: &LanguageIdentifier,
        text_id: &str,
        args: Option<&HashMap<&str, FluentValue>>,
    ) -> Option<String> {
        for l in self.fallbacks.get(lang).expect("language not found") {
            if let Some(val) = self.lookup_single_language(l, text_id, args) {
                return Some(val);
            }
        }

        None
    }
}

impl Loader for SimpleLoader {
    // Traverse the fallback chain,
    fn lookup(
        &self,
        lang: &LanguageIdentifier,
        text_id: &str,
        args: Option<&HashMap<&str, FluentValue>>,
    ) -> String {
        for l in self.fallbacks.get(lang).expect("language not found") {
            if let Some(val) = self.lookup_single_language(l, text_id, args) {
                return val;
            }
        }
        if *lang != self.fallback {
            if let Some(val) = self.lookup_single_language(&self.fallback, text_id, args) {
                return val;
            }
        }
        format!("Unknown localization {}", text_id)
    }
}

fn read_from_file<P: AsRef<Path>>(filename: P) -> io::Result<FluentResource> {
    let mut file = File::open(filename)?;
    let mut string = String::new();

    file.read_to_string(&mut string)?;

    Ok(FluentResource::try_new(string).expect("File did not parse!"))
}

fn read_from_dir<P: AsRef<Path>>(dirname: P) -> io::Result<Vec<FluentResource>> {
    let mut result = Vec::new();
    for dir_entry in read_dir(dirname)? {
        let entry = dir_entry?;

        // Prevent loading non-FTL files as translations, such as VIM temporary files.
        if entry.path().extension().and_then(|e| e.to_str()) != Some("ftl") {
            continue;
        }

        let resource = read_from_file(entry.path())?;
        result.push(resource);
    }
    Ok(result)
}

pub fn create_bundle(
    lang: LanguageIdentifier,
    resources: &'static [FluentResource],
    core_resource: Option<&'static FluentResource>,
    customizer: &impl Fn(&mut FluentBundle<&'static FluentResource>),
) -> FluentBundle<&'static FluentResource> {
    let mut bundle: FluentBundle<&'static FluentResource> = FluentBundle::new(&[lang]);
    if let Some(core) = core_resource {
        bundle
            .add_resource(core)
            .expect("Failed to add core resource to bundle");
    }
    for res in resources {
        bundle
            .add_resource(res)
            .expect("Failed to add FTL resources to the bundle.");
    }

    customizer(&mut bundle);
    bundle
}

pub fn build_resources(dir: &str) -> HashMap<LanguageIdentifier, Vec<FluentResource>> {
    let mut all_resources = HashMap::new();
    let entries = read_dir(dir).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            if let Ok(lang) = entry.file_name().into_string() {
                let resources = read_from_dir(entry.path()).unwrap();
                all_resources.insert(lang.parse().unwrap(), resources);
            }
        }
    }
    all_resources
}

pub fn build_bundles(
    resources: &'static HashMap<LanguageIdentifier, Vec<FluentResource>>,
    core_resource: Option<&'static FluentResource>,
    customizer: impl Fn(&mut FluentBundle<&'static FluentResource>),
) -> HashMap<LanguageIdentifier, FluentBundle<&'static FluentResource>> {
    let mut bundles = HashMap::new();
    for (k, v) in resources.iter() {
        bundles.insert(
            k.clone(),
            create_bundle(k.clone(), v, core_resource, &customizer),
        );
    }
    bundles
}

pub fn load_core_resource(path: &str) -> FluentResource {
    read_from_file(path).expect("cannot find core resource")
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_bundle::concurrent::FluentBundle;
    use std::error::Error;

    #[test]
    fn test_load_from_dir() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("core.ftl"), "foo = bar\n".as_bytes())?;
        std::fs::write(dir.path().join("other.ftl"), "bar = baz\n".as_bytes())?;
        std::fs::write(dir.path().join("invalid.txt"), "baz = foo\n".as_bytes())?;
        std::fs::write(dir.path().join(".binary_file.swp"), [0, 1, 2, 3, 4, 5])?;

        let result = read_from_dir(dir.path())?;
        assert_eq!(2, result.len()); // Doesn't include the binary file or the txt file

        let mut bundle = FluentBundle::new(&[unic_langid::langid!("en-US")]);
        for resource in &result {
            bundle.add_resource(resource).unwrap();
        }

        let mut errors = Vec::new();

        // Ensure the correct files were loaded
        assert_eq!(
            "bar",
            bundle.format_pattern(
                bundle.get_message("foo").and_then(|m| m.value).unwrap(),
                None,
                &mut errors
            )
        );

        assert_eq!(
            "baz",
            bundle.format_pattern(
                bundle.get_message("bar").and_then(|m| m.value).unwrap(),
                None,
                &mut errors
            )
        );
        assert_eq!(None, bundle.get_message("baz")); // The extension was txt

        Ok(())
    }
}
