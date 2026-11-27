#include "Core.h"

#include "nexusfm_ffi.h"

#include <QByteArray>

namespace nfm {

bool loadSettings(Settings &out)
{
    NfmConfig config{};
    const bool ok = nfm_config_load(&config);
    out.showHidden = config.show_hidden;
    out.sortKey = static_cast<SortKey>(config.sort_key);
    out.descending = config.descending;
    return ok;
}

ReadResult readDirectory(const QString &path, const Settings &settings)
{
    ReadResult result;

    const QByteArray utf8 = path.toUtf8();
    const NfmConfig config{
        settings.showHidden,
        static_cast<std::uint8_t>(settings.sortKey),
        settings.descending,
    };

    NfmListing *listing = nfm_listing_open(utf8.constData(), config);
    if (!listing) {
        result.error = QStringLiteral("Invalid path");
        return result;
    }

    if (const char *error = nfm_listing_error(listing)) {
        result.error = QString::fromUtf8(error);
        nfm_listing_free(listing);
        return result;
    }

    const std::size_t count = nfm_listing_count(listing);
    result.entries.reserve(static_cast<qsizetype>(count));
    for (std::size_t i = 0; i < count; ++i) {
        NfmEntry raw{};
        if (!nfm_listing_get(listing, i, &raw))
            continue;

        FileEntry entry;
        entry.name = QString::fromUtf8(raw.name);
        entry.path = QString::fromUtf8(raw.path);
        entry.kind = static_cast<EntryKind>(raw.kind);
        entry.size = raw.size;
        entry.hasModified = raw.has_modified;
        entry.modifiedUnix = raw.modified_unix;
        result.entries.append(entry);
    }

    nfm_listing_free(listing);
    result.ok = true;
    return result;
}

namespace ops {

namespace {

// Converts a C error message into a QString and releases it.
QString takeError(char *message)
{
    if (!message)
        return {};
    const QString text = QString::fromUtf8(message);
    nfm_string_free(message);
    return text;
}

} // namespace

QString createFolder(const QString &parent, const QString &name)
{
    return takeError(nfm_create_directory(parent.toUtf8().constData(), name.toUtf8().constData()));
}

QString rename(const QString &path, const QString &newName)
{
    return takeError(nfm_rename(path.toUtf8().constData(), newName.toUtf8().constData()));
}

QString copyInto(const QString &src, const QString &destDir)
{
    return takeError(nfm_copy(src.toUtf8().constData(), destDir.toUtf8().constData()));
}

QString moveInto(const QString &src, const QString &destDir)
{
    return takeError(nfm_move(src.toUtf8().constData(), destDir.toUtf8().constData()));
}

QString moveToTrash(const QString &path)
{
    return takeError(nfm_move_to_trash(path.toUtf8().constData()));
}

} // namespace ops

} // namespace nfm
