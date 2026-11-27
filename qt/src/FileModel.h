// Table model for a directory listing. Order comes from the Rust core, so the
// model does not sort on its own.
#pragma once

#include "Core.h"

#include <QAbstractTableModel>
#include <QFileIconProvider>

class FileModel : public QAbstractTableModel
{
    Q_OBJECT

public:
    enum Column { NameColumn = 0, SizeColumn, TypeColumn, ModifiedColumn, ColumnCount };

    explicit FileModel(QObject *parent = nullptr);

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    int columnCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;
    QVariant headerData(int section, Qt::Orientation orientation, int role = Qt::DisplayRole) const override;

    void setEntries(QVector<nfm::FileEntry> entries);
    const nfm::FileEntry *entryAt(int row) const;

private:
    QVector<nfm::FileEntry> m_entries;
    QFileIconProvider m_icons;
};
