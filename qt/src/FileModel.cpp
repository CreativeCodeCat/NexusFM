// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

#include "FileModel.h"

#include <QDateTime>
#include <QLocale>

FileModel::FileModel(QObject *parent)
    : QAbstractTableModel(parent)
{
}

int FileModel::rowCount(const QModelIndex &parent) const
{
    return parent.isValid() ? 0 : static_cast<int>(m_entries.size());
}

int FileModel::columnCount(const QModelIndex &parent) const
{
    return parent.isValid() ? 0 : ColumnCount;
}

QVariant FileModel::data(const QModelIndex &index, int role) const
{
    if (!index.isValid() || index.row() < 0 || index.row() >= m_entries.size())
        return {};

    const nfm::FileEntry &entry = m_entries.at(index.row());

    if (role == Qt::DecorationRole && index.column() == NameColumn) {
        return m_icons.icon(entry.isDirectory() ? QFileIconProvider::Folder : QFileIconProvider::File);
    }

    if (role == Qt::TextAlignmentRole && index.column() == SizeColumn)
        return QVariant(Qt::AlignRight | Qt::AlignVCenter);

    if (role != Qt::DisplayRole)
        return {};

    switch (index.column()) {
    case NameColumn:
        return entry.name;
    case SizeColumn:
        if (entry.isDirectory())
            return {};
        return QLocale().formattedDataSize(static_cast<qint64>(entry.size));
    case TypeColumn:
        switch (entry.kind) {
        case nfm::EntryKind::Directory:
            return QStringLiteral("Folder");
        case nfm::EntryKind::Symlink:
            return QStringLiteral("Link");
        case nfm::EntryKind::File:
            return QStringLiteral("File");
        case nfm::EntryKind::Other:
            return QStringLiteral("Other");
        }
        return {};
    case ModifiedColumn:
        if (!entry.hasModified)
            return {};
        return QLocale().toString(QDateTime::fromSecsSinceEpoch(entry.modifiedUnix), QLocale::ShortFormat);
    }
    return {};
}

QVariant FileModel::headerData(int section, Qt::Orientation orientation, int role) const
{
    if (orientation != Qt::Horizontal || role != Qt::DisplayRole)
        return {};

    switch (section) {
    case NameColumn:
        return tr("Name");
    case SizeColumn:
        return tr("Size");
    case TypeColumn:
        return tr("Type");
    case ModifiedColumn:
        return tr("Modified");
    }
    return {};
}

void FileModel::setEntries(QVector<nfm::FileEntry> entries)
{
    beginResetModel();
    m_entries = std::move(entries);
    endResetModel();
}

const nfm::FileEntry *FileModel::entryAt(int row) const
{
    if (row < 0 || row >= m_entries.size())
        return nullptr;
    return &m_entries.at(row);
}
