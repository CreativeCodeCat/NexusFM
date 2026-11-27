// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! User configuration for NexusFM.
//!
//! Settings live in `$XDG_CONFIG_HOME/nexusfm/config.toml`, falling back to
//! `$HOME/.config/nexusfm/config.toml`. A missing file is not an error: the
//! defaults apply. Unknown keys are ignored so older builds can read newer files.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use std::time::{Duration, Instant};

use mlua::{HookTriggers, Lua, LuaOptions, LuaSerdeExt, StdLib, Value, VmState};
use nexusfm_core::SortOrder;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Name of the TOML configuration file inside the NexusFM config directory.
pub const CONFIG_FILE_NAME: &str = "config.toml";

/// How long a Lua configuration script may run before it is stopped.
const SCRIPT_TIME_LIMIT: Duration = Duration::from_secs(2);

/// Name of the Lua configuration file. When present it takes priority over
/// [`CONFIG_FILE_NAME`].
pub const LUA_CONFIG_FILE_NAME: &str = "config.lua";

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
    #[error("cannot read Lua configuration {}: {source}", path.display())]
    LuaRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid Lua configuration {}: {message}", path.display())]
    Lua { path: PathBuf, message: String },
}

/// How a folder's entries are laid out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewMode {
    /// One row per entry, with name, size, type and modified columns.
    #[default]
    Detailed,
    /// Names in columns, filling down then across.
    Compact,
    /// Large icons with names underneath, in a grid.
    Icon,
}

/// Settings that apply across all windows.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Whether file views show entries whose names start with a dot.
    pub show_hidden: bool,
    /// Sort order used when a directory has no saved preference.
    pub sort: SortOrder,
    /// Layout used for file views.
    pub view: ViewMode,
}

impl Config {
    /// Parses configuration from TOML text. `origin` names the source in errors.
    pub fn parse(text: &str, origin: &Path) -> Result<Self, ConfigError> {
        toml::from_str(text).map_err(|source| ConfigError::Parse {
            path: origin.to_path_buf(),
            source,
        })
    }

    /// Parses a Lua configuration script. The script must return a table
    /// with the same keys as the TOML file, for example
    /// `return { show_hidden = true, view = "icon" }`.
    ///
    /// The script runs in a restricted Lua state: only the math, string, table
    /// and UTF-8 libraries are available, and the functions that read files or
    /// load other code (`dofile`, `loadfile`, `require`, `load`) are removed.
    pub fn parse_lua(text: &str, origin: &Path) -> Result<Self, ConfigError> {
        let lua_error = |err: mlua::Error| ConfigError::Lua {
            path: origin.to_path_buf(),
            message: err.to_string(),
        };

        let lua = Lua::new_with(
            StdLib::MATH | StdLib::STRING | StdLib::TABLE | StdLib::UTF8,
            LuaOptions::default(),
        )
        .map_err(lua_error)?;
        for name in ["dofile", "loadfile", "require", "load"] {
            lua.globals().raw_set(name, Value::Nil).map_err(lua_error)?;
        }

        // Stop a script that runs too long, such as an accidental infinite loop.
        let started = Instant::now();
        lua.set_hook(
            HookTriggers {
                every_nth_instruction: Some(10_000),
                ..Default::default()
            },
            move |_lua, _debug| {
                if started.elapsed() > SCRIPT_TIME_LIMIT {
                    Err(mlua::Error::runtime("the script ran for too long"))
                } else {
                    Ok(VmState::Continue)
                }
            },
        )
        .map_err(lua_error)?;

        let value: Value = lua
            .load(text)
            .set_name(origin.display().to_string())
            .eval()
            .map_err(lua_error)?;
        if matches!(value, Value::Nil) {
            return Ok(Self::default());
        }
        lua.from_value(value).map_err(lua_error)
    }

    /// Loads a Lua configuration script from `path`.
    pub fn load_lua(path: &Path) -> Result<Self, ConfigError> {
        let text = fs::read_to_string(path).map_err(|source| ConfigError::LuaRead {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse_lua(&text, path)
    }

    /// Loads the user's configuration from the XDG config directory.
    ///
    /// `config.lua` is used if it exists, then `config.toml`, otherwise the
    /// defaults. `xdg_config_home` and `home` are the environment values.
    pub fn load_user(
        xdg_config_home: Option<&OsStr>,
        home: Option<&OsStr>,
    ) -> Result<Self, ConfigError> {
        let Some(dir) = config_dir(xdg_config_home, home) else {
            return Ok(Self::default());
        };
        let lua_path = dir.join(LUA_CONFIG_FILE_NAME);
        if lua_path.is_file() {
            return Self::load_lua(&lua_path);
        }
        Self::load(&dir.join(CONFIG_FILE_NAME))
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
    Some(config_dir(xdg_config_home, home)?.join(CONFIG_FILE_NAME))
}

/// The NexusFM config directory: `$XDG_CONFIG_HOME/nexusfm`, or
/// `$HOME/.config/nexusfm`.
pub fn config_dir(xdg_config_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf> {
    let base = match xdg_config_home.filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(home.filter(|v| !v.is_empty())?).join(".config"),
    };
    Some(base.join("nexusfm"))
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
    fn reads_view_mode() {
        let config = Config::parse("view = \"icon\"\n", &origin()).unwrap();
        assert_eq!(config.view, ViewMode::Icon);
        assert_eq!(
            Config::parse("", &origin()).unwrap().view,
            ViewMode::Detailed
        );
        assert!(Config::parse("view = \"tiles\"\n", &origin()).is_err());
    }

    #[test]
    fn lua_config_sets_values() {
        let text = r#"
            local sort = { key = "size", descending = true }
            return { show_hidden = true, sort = sort, view = "icon" }
        "#;
        let config = Config::parse_lua(text, &origin()).unwrap();
        assert!(config.show_hidden);
        assert_eq!(config.sort.key, SortKey::Size);
        assert!(config.sort.descending);
        assert_eq!(config.view, ViewMode::Icon);
    }

    #[test]
    fn lua_config_can_compute_values() {
        let config = Config::parse_lua("return { show_hidden = (2 + 2) == 4 }", &origin()).unwrap();
        assert!(config.show_hidden);
    }

    #[test]
    fn lua_config_without_return_gives_defaults() {
        assert_eq!(
            Config::parse_lua("x = 1", &origin()).unwrap(),
            Config::default()
        );
    }

    #[test]
    fn lua_config_cannot_touch_files_or_load_code() {
        for text in [
            "return { show_hidden = io ~= nil }",
            "return { show_hidden = os ~= nil }",
            "return { show_hidden = dofile ~= nil }",
            "return { show_hidden = loadfile ~= nil }",
            "return { show_hidden = require ~= nil }",
            "return { show_hidden = load ~= nil }",
        ] {
            let config = Config::parse_lua(text, &origin()).unwrap();
            assert!(!config.show_hidden, "{text} should not expose the library");
        }
    }

    #[test]
    fn lua_errors_are_reported() {
        assert!(matches!(
            Config::parse_lua("return {", &origin()),
            Err(ConfigError::Lua { .. })
        ));
        assert!(Config::parse_lua("return { view = 'tiles' }", &origin()).is_err());
        assert!(Config::parse_lua("return os.exit(1)", &origin()).is_err());
        assert!(Config::parse_lua("while true do end", &origin()).is_err());
    }

    #[test]
    fn lua_config_takes_priority_over_toml() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("nexusfm");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join(CONFIG_FILE_NAME), "show_hidden = false\n").unwrap();
        fs::write(
            dir.join(LUA_CONFIG_FILE_NAME),
            "return { show_hidden = true }",
        )
        .unwrap();

        let config = Config::load_user(Some(root.path().as_os_str()), None).unwrap();
        assert!(config.show_hidden);
    }

    #[test]
    fn toml_is_used_without_lua_and_defaults_without_either() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            Config::load_user(Some(root.path().as_os_str()), None).unwrap(),
            Config::default()
        );
        let dir = root.path().join("nexusfm");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join(CONFIG_FILE_NAME), "view = \"compact\"\n").unwrap();
        let config = Config::load_user(Some(root.path().as_os_str()), None).unwrap();
        assert_eq!(config.view, ViewMode::Compact);
    }

    #[test]
    fn config_dir_follows_xdg_rules() {
        assert_eq!(
            config_dir(Some(OsStr::new("/x")), Some(OsStr::new("/h"))),
            Some(PathBuf::from("/x/nexusfm"))
        );
        assert_eq!(
            config_dir(None, Some(OsStr::new("/h"))),
            Some(PathBuf::from("/h/.config/nexusfm"))
        );
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
