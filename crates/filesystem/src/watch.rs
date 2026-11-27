// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! Change notifications for a folder.

use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use nexusfm_core::{NexusError, Result};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

/// Watches one folder (not its subfolders) for entries being added, removed or changed.
pub struct DirectoryWatcher {
    // Kept alive for as long as the watch is needed.
    _watcher: RecommendedWatcher,
    events: Receiver<notify::Result<notify::Event>>,
}

impl DirectoryWatcher {
    /// Starts watching `dir`.
    pub fn new(dir: &Path) -> Result<Self> {
        let (sender, events) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |event| {
            // The receiver may be gone if the window closed; nothing to do then.
            let _ = sender.send(event);
        })
        .map_err(|err| NexusError::Message(format!("cannot watch {}: {err}", dir.display())))?;
        watcher
            .watch(dir, RecursiveMode::NonRecursive)
            .map_err(|err| NexusError::Message(format!("cannot watch {}: {err}", dir.display())))?;
        Ok(Self {
            _watcher: watcher,
            events,
        })
    }

    /// Whether anything in the folder changed since the last call. Events are
    /// drained, so each change is reported once. Events that cannot be read are
    /// ignored.
    pub fn take_changes(&self) -> bool {
        let mut changed = false;
        loop {
            match self.events.try_recv() {
                Ok(_) => changed = true,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return changed,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Polls until a change is seen or two seconds pass.
    fn wait_for_change(watcher: &DirectoryWatcher) -> bool {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if watcher.take_changes() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    #[test]
    fn reports_new_files() {
        let dir = tempfile::tempdir().unwrap();
        let watcher = DirectoryWatcher::new(dir.path()).unwrap();
        assert!(!watcher.take_changes());
        std::fs::write(dir.path().join("new.txt"), b"x").unwrap();
        assert!(wait_for_change(&watcher));
    }

    #[test]
    fn reports_removals() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("old.txt");
        std::fs::write(&file, b"x").unwrap();
        let watcher = DirectoryWatcher::new(dir.path()).unwrap();
        std::fs::remove_file(&file).unwrap();
        assert!(wait_for_change(&watcher));
    }

    #[test]
    fn missing_folder_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(DirectoryWatcher::new(&dir.path().join("missing")).is_err());
    }
}
