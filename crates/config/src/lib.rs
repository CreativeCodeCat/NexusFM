//! User configuration for NexusFM.
//!
//! Settings live in `$XDG_CONFIG_HOME/nexusfm/config.toml`, falling back to
//! `$HOME/.config/nexusfm/config.toml`. A missing file is not an error: the
//! defaults apply. Unknown keys are ignored so older builds can read newer files.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nexusfm_core::SortOrder;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Name of the configuration file inside the NexusFM config directory.
pub const CONFIG_FILE_NAME: &str = "config.toml";

/// Errors from reading or parsing configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("cannot read configuration file {}: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid configuration file {}: {source}", path.display())]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

/// Settings that apply across all windows.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Whether file views show entries whose names start with a dot.
    pub show_hidden: bool,
    /// Sort order used when a directory has no saved preference.
    pub sort: SortOrder,
}

impl Config {
    /// Parses configuration from TOML text. `origin` names the source in errors.
    pub fn parse(text: &str, origin: &Path) -> Result<Self, ConfigError> {
        toml::from_str(text).map_err(|source| ConfigError::Parse {
            path: origin.to_path_buf(),
            source,
        })
    }

    /// Loads configuration from `path`. A missing file yields the defaults.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match fs::read_to_string(path) {
            Ok(text) => Self::parse(&text, path),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Read {
                path: path.to_path_buf(),
                source,
            }),
        }
    }
}

/// Resolves the configuration file path from the XDG environment variables.
///
/// Takes the variables as arguments so callers and tests control the lookup.
/// Returns `None` when neither `XDG_CONFIG_HOME` nor `HOME` is usable.
pub fn config_file_path(xdg_config_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf> {
    let base = match xdg_config_home.filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(home.filter(|v| !v.is_empty())?).join(".config"),
    };
    Some(base.join("nexusfm").join(CONFIG_FILE_NAME))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexusfm_core::SortKey;

    fn origin() -> PathBuf {
        PathBuf::from("test.toml")
    }

    #[test]
    fn empty_text_gives_defaults() {
        assert_eq!(Config::parse("", &origin()).unwrap(), Config::default());
    }

    #[test]
    fn reads_known_keys() {
        let text = "show_hidden = true\n\n[sort]\nkey = \"size\"\ndescending = true\n";
        let config = Config::parse(text, &origin()).unwrap();
        assert!(config.show_hidden);
        assert_eq!(config.sort.key, SortKey::Size);
        assert!(config.sort.descending);
    }

    #[test]
    fn ignores_unknown_keys() {
        let config = Config::parse("future_setting = 3\n", &origin()).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn rejects_malformed_toml() {
        let err = Config::parse("show_hidden = ", &origin()).unwrap_err();
        assert!(matches!(err, ConfigError::Parse { .. }));
    }

    #[test]
    fn rejects_wrong_value_type() {
        assert!(Config::parse("show_hidden = \"yes\"", &origin()).is_err());
    }

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(&dir.path().join("absent.toml")).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn loads_file_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILE_NAME);
        fs::write(&path, "show_hidden = true\n").unwrap();
        assert!(Config::load(&path).unwrap().show_hidden);
    }

    #[test]
    fn xdg_config_home_takes_precedence() {
        let path = config_file_path(Some(OsStr::new("/x")), Some(OsStr::new("/home/u")));
        assert_eq!(path, Some(PathBuf::from("/x/nexusfm/config.toml")));
    }

    #[test]
    fn falls_back_to_home_dot_config() {
        let path = config_file_path(None, Some(OsStr::new("/home/u")));
        assert_eq!(
            path,
            Some(PathBuf::from("/home/u/.config/nexusfm/config.toml"))
        );
    }

    #[test]
    fn empty_xdg_variable_is_ignored() {
        let path = config_file_path(Some(OsStr::new("")), Some(OsStr::new("/home/u")));
        assert_eq!(
            path,
            Some(PathBuf::from("/home/u/.config/nexusfm/config.toml"))
        );
    }

    #[test]
    fn no_environment_gives_no_path() {
        assert_eq!(config_file_path(None, None), None);
    }
}
