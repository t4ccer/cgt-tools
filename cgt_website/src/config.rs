use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, Visitor},
};
use std::{
    collections::BTreeMap,
    fmt, fs, io,
    path::{Path, PathBuf},
};

/// What goes on the site besides the pages written in Rust, read from a TOML file. Paths in it
/// are relative to the file
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Guides in the order they are listed in on the site, as Markdown files or Jupyter notebooks
    #[serde(default)]
    pub guides: Vec<PathBuf>,
    /// Models to play each game against, by the name of the game, see [`Models`]
    #[serde(default)]
    pub play: BTreeMap<String, Models>,
}

/// Model files by the names the site shows them under, in the order they are written in, which is
/// the order of the choices on the site, the first model being the default. A map would lose the
/// order, so this reads the entries one by one, which `toml` hands over in order with its
/// `preserve_order` feature
#[derive(Default)]
pub struct Models(pub Vec<(String, Source)>);

/// Where a model file comes from
#[derive(Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum Source {
    /// A file that the site serves itself
    Path(PathBuf),
    /// A file that the page loads from another site, which has to allow that with the
    /// `Access-Control-Allow-Origin` header
    Url(String),
}

impl<'de> Deserialize<'de> for Models {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Models, D::Error> {
        struct Entries;

        impl<'de> Visitor<'de> for Entries {
            type Value = Models;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a table of models")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Models, A::Error> {
                let mut models = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    models.push(entry);
                }
                Ok(Models(models))
            }
        }

        deserializer.deserialize_map(Entries)
    }
}

fn invalid(path: &Path, message: impl fmt::Display) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{}: {message}", path.display()),
    )
}

impl Config {
    /// # Errors
    ///
    /// When the file cannot be read or is not a valid configuration
    pub fn read(path: &Path) -> io::Result<Config> {
        let text = fs::read_to_string(path).map_err(|err| invalid(path, err))?;
        let mut config: Config = toml::from_str(&text).map_err(|err| invalid(path, err))?;
        let dir = path.parent().unwrap_or_else(|| Path::new(""));
        for guide in &mut config.guides {
            *guide = dir.join(&*guide);
        }
        for (name, source) in config.play.values_mut().flat_map(|models| &mut models.0) {
            // The name ends up in the address of the file
            if name.is_empty()
                || !name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(invalid(
                    path,
                    format!(
                        "the model name `{name}` has other characters than letters, digits, `-` and `_`"
                    ),
                ));
            }
            if let Source::Path(file) = source {
                *file = dir.join(&*file);
            }
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_keep_their_order() {
        let config: Config = toml::from_str(
            r#"
            guides = ["a.md", "b.ipynb"]

            [play.quelhas]
            strong = { url = "https://example.com/strong.bin" }
            quick = { path = "quick.bin" }
            "#,
        )
        .unwrap();
        assert_eq!(
            config.guides,
            [PathBuf::from("a.md"), PathBuf::from("b.ipynb")]
        );
        let models = &config.play["quelhas"].0;
        assert_eq!(models.len(), 2);
        assert!(
            matches!(&models[0], (name, Source::Url(url)) if name == "strong" && url == "https://example.com/strong.bin")
        );
        assert!(
            matches!(&models[1], (name, Source::Path(path)) if name == "quick" && path == Path::new("quick.bin"))
        );
    }

    #[test]
    fn sources_name_one_place() {
        for source in [
            r#"m = { path = "a", url = "b" }"#,
            r"m = {}",
            r#"m = { file = "a" }"#,
        ] {
            assert!(toml::from_str::<Models>(source).is_err(), "{source}");
        }
    }
}
