// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

//! Mounted and mountable devices, through GIO's volume monitor.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use gio::prelude::*;
use nexusfm_core::{NexusError, Result};

/// What kind of device a sidebar entry stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    /// A local disk, USB stick or other drive.
    Drive,
    /// A removable drive, such as a USB stick or memory card.
    Removable,
    /// A network or other remote mount, such as an SMB share.
    Network,
}

/// A device shown in the sidebar: either mounted, or a volume that can be mounted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// Identifies the device across refreshes: `mount:<root uri>` or `volume:<name>`.
    pub key: String,
    pub name: String,
    pub kind: DeviceKind,
    pub mounted: bool,
    pub can_unmount: bool,
    /// The root URI of a mounted device.
    pub root: Option<String>,
    pub free_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
}

/// The devices GIO knows about right now.
pub fn list_devices() -> Vec<Device> {
    let monitor = gio::VolumeMonitor::get();
    let mut devices = Vec::new();

    for mount in monitor.mounts() {
        let root = mount.root();
        let uri = root.uri().to_string();
        let remote = !uri.starts_with("file:");
        let drive = mount.drive();
        let removable = drive.as_ref().is_some_and(|drive| drive.is_removable());
        // Skip internal mounts such as the system root, trash and recent files.
        if !remote && drive.is_none() {
            continue;
        }
        let kind = if remote {
            DeviceKind::Network
        } else if removable {
            DeviceKind::Removable
        } else {
            DeviceKind::Drive
        };
        let (free_bytes, total_bytes) = space(&root);
        devices.push(Device {
            key: format!("mount:{uri}"),
            name: mount.name().to_string(),
            kind,
            mounted: true,
            can_unmount: mount.can_unmount(),
            root: Some(uri),
            free_bytes,
            total_bytes,
        });
    }

    for volume in monitor.volumes() {
        if volume.get_mount().is_some() || !volume.can_mount() {
            continue;
        }
        let removable = volume.drive().is_some_and(|drive| drive.is_removable());
        devices.push(Device {
            key: format!("volume:{}", volume.name()),
            name: volume.name().to_string(),
            kind: if removable {
                DeviceKind::Removable
            } else {
                DeviceKind::Drive
            },
            mounted: false,
            can_unmount: false,
            root: None,
            free_bytes: None,
            total_bytes: None,
        });
    }

    devices
}

/// Mounts the volume named by `key` (see [`Device::key`]).
pub fn mount_device(key: &str) -> Result<()> {
    let name = key
        .strip_prefix("volume:")
        .ok_or_else(|| NexusError::Message(format!("{key} is not a device that can be mounted")))?;
    let volume = gio::VolumeMonitor::get()
        .volumes()
        .into_iter()
        .find(|volume| volume.name() == name)
        .ok_or_else(|| NexusError::Message(format!("{name} is no longer connected")))?;

    let result = wait_for(|done| {
        volume.mount(
            gio::MountMountFlags::NONE,
            None::<&gio::MountOperation>,
            None::<&gio::Cancellable>,
            done,
        )
    });
    result.map_err(|err| NexusError::Message(format!("cannot mount {name}: {}", err.message())))
}

/// Unmounts the device named by `key` (see [`Device::key`]).
pub fn unmount_device(key: &str) -> Result<()> {
    let root = key
        .strip_prefix("mount:")
        .ok_or_else(|| NexusError::Message(format!("{key} is not a mounted device")))?;
    let mount = gio::VolumeMonitor::get()
        .mounts()
        .into_iter()
        .find(|mount| mount.root().uri() == root)
        .ok_or_else(|| NexusError::Message(format!("{root} is not mounted")))?;
    let name = mount.name().to_string();

    let result = wait_for(|done| {
        mount.unmount_with_operation(
            gio::MountUnmountFlags::NONE,
            None::<&gio::MountOperation>,
            None::<&gio::Cancellable>,
            done,
        )
    });
    result.map_err(|err| NexusError::Message(format!("cannot unmount {name}: {}", err.message())))
}

/// Runs a GIO operation that reports back through a callback, and waits for it.
///
/// GIO delivers the callback on the default GLib main context, which the Qt
/// event loop does not run. So this pumps that context until the result arrives.
fn wait_for<F>(start: F) -> std::result::Result<(), gio::glib::Error>
where
    F: FnOnce(Box<dyn FnOnce(std::result::Result<(), gio::glib::Error>) + 'static>),
{
    let (sender, receiver) = mpsc::channel();
    start(Box::new(move |outcome| {
        let _ = sender.send(outcome);
    }));

    let context = gio::glib::MainContext::default();
    let _owner = context.acquire();
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    loop {
        if let Ok(outcome) = receiver.try_recv() {
            return outcome;
        }
        if std::time::Instant::now() > deadline {
            return Err(gio::glib::Error::new(
                gio::IOErrorEnum::TimedOut,
                "the operation timed out",
            ));
        }
        context.iteration(false);
        thread::sleep(Duration::from_millis(10));
    }
}

/// Free and total bytes on the filesystem at `root`, when the device reports them.
fn space(root: &gio::File) -> (Option<u64>, Option<u64>) {
    let Ok(info) = root.query_filesystem_info(
        "filesystem::size,filesystem::free",
        None::<&gio::Cancellable>,
    ) else {
        return (None, None);
    };
    let read = |attribute: &str| {
        if info.has_attribute(attribute) {
            Some(info.attribute_uint64(attribute))
        } else {
            None
        }
    };
    (read("filesystem::free"), read("filesystem::size"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_devices_does_not_fail() {
        // Whatever this machine has connected, listing must return without panicking.
        let _ = list_devices();
    }

    #[test]
    fn unknown_devices_are_reported_not_guessed() {
        assert!(mount_device("volume:no-such-device-here").is_err());
        assert!(mount_device("mount:file:///").is_err());
        assert!(unmount_device("volume:anything").is_err());
        assert!(unmount_device("mount:smb://no-such-server/share").is_err());
    }
}
