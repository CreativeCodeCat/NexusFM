// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! User configuration for NexusFM, written in Lua.
//!
//! Settings live in `$XDG_CONFIG_HOME/nexusfm/config.lua`, falling back to
//! `$HOME/.config/nexusfm/config.lua`. The file must return a table, for example
//! `return { show_hidden = true, view = "icon" }`. A missing file gives the
//! defaults, and settings left out of the table keep their defaults.
//!
//! The script runs in a restricted Lua state with no file or process access,
//! and it is stopped if it runs for too long.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use mlua::{HookTriggers, Lua, LuaOptions, LuaSerdeExt, StdLib, Value, VmState};
use nexusfm_core::{SortKey, SortOrder};
use serde::Deserialize;
use thiserror::Error;

pub mod window;
pub use window::WindowState;

/// Name of the configuration file inside the NexusFM config directory.
pub const CONFIG_FILE_NAME: &str = "config.lua";

/// Name of the old TOML configuration file. It is no longer read; if it is
/// present without a `config.lua`, the user is told to move its settings.
const LEGACY_TOML_FILE_NAME: &str = "config.toml";

/// Name of the copy kept of the previous `config.lua` when settings are saved.
const BACKUP_FILE_NAME: &str = "config.lua.bak";

/// How long a configuration script may run before it is stopped.
const SCRIPT_TIME_LIMIT: Duration = Duration::from_secs(2);

/// Errors from reading or running the configuration script.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("cannot read configuration file {}: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid configuration file {}: {message}", path.display())]
    Lua { path: PathBuf, message: String },
    #[error("cannot write configuration file {}: {source}", path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(
        "{} is no longer read; move its settings into config.lua as a returned table",
        path.display()
    )]
    LegacyToml { path: PathBuf },
}

/// How a folder's entries are laid out.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
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
    /// Runs a configuration script and reads the table it returns.
    ///
    /// The script runs with only the math, string, table and UTF-8 libraries.
    /// The functions that read files or load other code (`dofile`, `loadfile`,
    /// `require`, `load`) are removed. `origin` names the script in errors.
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

    /// Runs the configuration script at `path`.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse_lua(&text, path)
    }

    /// Renders the settings as a Lua script that reads back to the same values.
    pub fn to_lua(&self) -> String {
        let key = match self.sort.key {
            SortKey::Name => "name",
            SortKey::Size => "size",
            SortKey::Modified => "modified",
        };
        let view = match self.view {
            ViewMode::Detailed => "detailed",
            ViewMode::Compact => "compact",
            ViewMode::Icon => "icon",
        };
        let lines = [
            "-- Written by NexusFM when a setting changes. Your original file is".to_string(),
            "-- kept in config.lua.bak.".to_string(),
            "return {".to_string(),
            format!("  show_hidden = {},", self.show_hidden),
            format!("  view = \"{view}\","),
            "  sort = {".to_string(),
            format!("    key = \"{key}\","),
            format!("    descending = {},", self.sort.descending),
            "  },".to_string(),
            "}".to_string(),
        ];
        lines.join("\n") + "\n"
    }

    /// Writes the settings to `config.lua` in the XDG config directory.
    ///
    /// The first time the file is rewritten, the original is copied to
    /// `config.lua.bak` and kept there. Later saves do not change the backup. The
    /// new file is written to a temporary name and renamed into place. Comments
    /// and custom code in the current file are not kept in `config.lua`.
    pub fn save_user(
        &self,
        xdg_config_home: Option<&OsStr>,
        home: Option<&OsStr>,
    ) -> Result<(), ConfigError> {
        let Some(dir) = config_dir(xdg_config_home, home) else {
            return Ok(());
        };
        let path = dir.join(CONFIG_FILE_NAME);
        let write_error = |source: io::Error| ConfigError::Write {
            path: path.clone(),
            source,
        };

        fs::create_dir_all(&dir).map_err(write_error)?;
        let backup = dir.join(BACKUP_FILE_NAME);
        if path.is_file() && !backup.exists() {
            fs::copy(&path, &backup).map_err(write_error)?;
        }
        let temp = dir.join(format!("{CONFIG_FILE_NAME}.tmp"));
        fs::write(&temp, self.to_lua()).map_err(write_error)?;
        fs::rename(&temp, &path).map_err(write_error)
    }

    /// Loads the user's configuration from the XDG config directory.
    ///
    /// A missing `config.lua` gives the defaults. If only the old `config.toml`
    /// exists, this returns [`ConfigError::LegacyToml`] so the caller can say so.
    /// `xdg_config_home` and `home` are the environment values.
    pub fn load_user(
        xdg_config_home: Option<&OsStr>,
        home: Option<&OsStr>,
    ) -> Result<Self, ConfigError> {
        let Some(dir) = config_dir(xdg_config_home, home) else {
            return Ok(Self::default());
        };
        let lua_path = dir.join(CONFIG_FILE_NAME);
        if lua_path.is_file() {
            return Self::load(&lua_path);
        }
        let legacy = dir.join(LEGACY_TOML_FILE_NAME);
        if legacy.is_file() {
            return Err(ConfigError::LegacyToml { path: legacy });
        }
        Ok(Self::default())
    }
}

/// The NexusFM config directory: `$XDG_CONFIG_HOME/nexusfm`, or
/// `$HOME/.config/nexusfm`. Returns `None` when neither variable is set.
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

    fn origin() -> PathBuf {
        PathBuf::from("config.lua")
    }

    #[test]
    fn reads_all_settings() {
        let text = r#"
            local sort = { key = "size", descending = true }
            return { show_hidden = true, sort = sort, view = "compact" }
        "#;
        let config = Config::parse_lua(text, &origin()).unwrap();
        assert!(config.show_hidden);
        assert_eq!(
            config.sort,
            SortOrder {
                key: SortKey::Size,
                descending: true
            }
        );
        assert_eq!(config.view, ViewMode::Compact);
    }

    #[test]
    fn missing_settings_keep_defaults() {
        let config = Config::parse_lua("return { view = 'icon' }", &origin()).unwrap();
        assert_eq!(config.view, ViewMode::Icon);
        assert!(!config.show_hidden);
        assert_eq!(config.sort, SortOrder::default());
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let config = Config::parse_lua("return { future_setting = 3 }", &origin()).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn script_can_compute_values() {
        let config = Config::parse_lua("return { show_hidden = (2 + 2) == 4 }", &origin()).unwrap();
        assert!(config.show_hidden);
    }

    #[test]
    fn script_without_return_gives_defaults() {
        assert_eq!(
            Config::parse_lua("x = 1", &origin()).unwrap(),
            Config::default()
        );
    }

    #[test]
    fn wrong_value_types_are_errors() {
        assert!(Config::parse_lua("return { show_hidden = 'yes' }", &origin()).is_err());
        assert!(Config::parse_lua("return { view = 'tiles' }", &origin()).is_err());
    }

    #[test]
    fn script_cannot_touch_files_or_load_code() {
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
    fn script_errors_are_reported() {
        assert!(matches!(
            Config::parse_lua("return {", &origin()),
            Err(ConfigError::Lua { .. })
        ));
        assert!(Config::parse_lua("return os.exit(1)", &origin()).is_err());
    }

    #[test]
    fn infinite_loop_is_stopped() {
        assert!(Config::parse_lua("while true do end", &origin()).is_err());
    }

    #[test]
    fn load_user_reads_config_lua() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("nexusfm");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join(CONFIG_FILE_NAME), "return { show_hidden = true }").unwrap();

        let config = Config::load_user(Some(root.path().as_os_str()), None).unwrap();
        assert!(config.show_hidden);
    }

    #[test]
    fn load_user_without_files_gives_defaults() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            Config::load_user(Some(root.path().as_os_str()), None).unwrap(),
            Config::default()
        );
    }

    #[test]
    fn legacy_toml_is_reported_not_read() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("nexusfm");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join(LEGACY_TOML_FILE_NAME), "show_hidden = true\n").unwrap();

        let err = Config::load_user(Some(root.path().as_os_str()), None).unwrap_err();
        assert!(matches!(err, ConfigError::LegacyToml { .. }));
    }

    #[test]
    fn to_lua_round_trips() {
        for config in [
            Config::default(),
            Config {
                show_hidden: true,
                view: ViewMode::Icon,
                sort: SortOrder {
                    key: SortKey::Modified,
                    descending: true,
                },
            },
        ] {
            let text = config.to_lua();
            assert_eq!(Config::parse_lua(&text, &origin()).unwrap(), config);
        }
    }

    #[test]
    fn save_writes_config_and_keeps_a_backup() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("nexusfm");
        let xdg = Some(root.path().as_os_str());

        // First save: nothing to back up yet.
        let first = Config {
            show_hidden: true,
            ..Config::default()
        };
        first.save_user(xdg, None).unwrap();
        assert!(!dir.join(BACKUP_FILE_NAME).exists());
        assert_eq!(Config::load_user(xdg, None).unwrap(), first);

        // Second save with a hand-written file: the original is backed up once.
        fs::write(
            dir.join(CONFIG_FILE_NAME),
            "-- my notes\nreturn { view = 'compact' }",
        )
        .unwrap();
        let second = Config {
            view: ViewMode::Icon,
            ..Config::default()
        };
        second.save_user(xdg, None).unwrap();
        assert_eq!(Config::load_user(xdg, None).unwrap(), second);
        let backup = fs::read_to_string(dir.join(BACKUP_FILE_NAME)).unwrap();
        assert!(backup.contains("-- my notes"));

        // Later saves leave the backup alone.
        let third = Config {
            show_hidden: true,
            ..Config::default()
        };
        third.save_user(xdg, None).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join(BACKUP_FILE_NAME)).unwrap(),
            backup
        );
        assert!(!dir.join(format!("{CONFIG_FILE_NAME}.tmp")).exists());
    }

    #[test]
    fn config_dir_follows_xdg_rules() {
        assert_eq!(
            config_dir(Some(OsStr::new("/x")), Some(OsStr::new("/h"))),
            Some(PathBuf::from("/x/nexusfm"))
        );
        assert_eq!(
            config_dir(Some(OsStr::new("")), Some(OsStr::new("/h"))),
            Some(PathBuf::from("/h/.config/nexusfm"))
        );
        assert_eq!(config_dir(None, None), None);
    }
}
