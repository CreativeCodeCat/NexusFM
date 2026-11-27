// Qt-side wrapper around the Rust core (crates/ffi).
#pragma once

#include <QString>
#include <QVector>

#include <cstdint>

namespace nfm {

enum class SortKey { Name = 0, Size = 1, Modified = 2 };

// Settings shared across the window, loaded from the user's config file.
struct Settings {
    bool showHidden = false;
    SortKey sortKey = SortKey::Name;
    bool descending = false;
};

// One directory entry, copied out of the Rust listing.
enum class EntryKind : std::uint8_t { File = 0, Directory = 1, Symlink = 2, Other = 3 };

struct FileEntry {
    QString name;
    QString path;
    EntryKind kind = EntryKind::File;
    quint64 size = 0;
    bool hasModified = false;
    qint64 modifiedUnix = 0;

    bool isDirectory() const { return kind == EntryKind::Directory; }
};

struct ReadResult {
    bool ok = false;
    QString error;
    QVector<FileEntry> entries;
};

// Loads the user configuration. Returns false if a config file exists but is
// invalid; `out` then holds the defaults.
bool loadSettings(Settings &out);

// Lists `path` with the given settings applied (hidden files, sorting).
ReadResult readDirectory(const QString &path, const Settings &settings);

} // namespace nfm
