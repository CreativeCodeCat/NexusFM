#include "MainWindow.h"

#include "FileModel.h"

#include <QAction>
#include <QApplication>
#include <QDesktopServices>
#include <QDir>
#include <QFileInfo>
#include <QHeaderView>
#include <QKeySequence>
#include <QModelIndexList>
#include <QLabel>
#include <QIcon>
#include <QInputDialog>
#include <QItemSelectionModel>
#include <QMenu>
#include <QLineEdit>
#include <QListWidget>
#include <QMenuBar>
#include <QMessageBox>
#include <QStandardPaths>
#include <QSplitter>
#include <QStatusBar>
#include <QToolBar>
#include <QTreeView>
#include <QUrl>

namespace {

// Sidebar entries for the standard user folders that exist on this system.
struct Place {
    QString label;
    QString path;
};

QVector<Place> defaultPlaces()
{
    const QVector<std::pair<QString, QStandardPaths::StandardLocation>> locations = {
        {QObject::tr("Home"), QStandardPaths::HomeLocation},
        {QObject::tr("Desktop"), QStandardPaths::DesktopLocation},
        {QObject::tr("Documents"), QStandardPaths::DocumentsLocation},
        {QObject::tr("Downloads"), QStandardPaths::DownloadLocation},
        {QObject::tr("Music"), QStandardPaths::MusicLocation},
        {QObject::tr("Pictures"), QStandardPaths::PicturesLocation},
        {QObject::tr("Videos"), QStandardPaths::MoviesLocation},
    };

    QVector<Place> places;
    for (const auto &[label, location] : locations) {
        const QString path = QStandardPaths::writableLocation(location);
        if (!path.isEmpty() && QFileInfo(path).isDir())
            places.append({label, path});
    }
    places.append({QObject::tr("File System"), QStringLiteral("/")});
    return places;
}

} // namespace

MainWindow::MainWindow(const QString &startPath, QWidget *parent)
    : QMainWindow(parent)
{
    if (!nfm::loadSettings(m_settings))
        QMessageBox::warning(this, tr("Configuration"),
                             tr("The configuration file could not be read. Default settings are in use."));

    setWindowTitle(tr("NexusFM"));
    resize(960, 600);

    setupActions();
    setupMenus();
    setupToolBar();
    setupCentral();
    setupStatusBar();

    m_showHiddenAction->setChecked(m_settings.showHidden);
    navigateTo(startPath);
}

void MainWindow::setupActions()
{
    m_backAction = new QAction(QIcon::fromTheme(QStringLiteral("go-previous")), tr("Back"), this);
    m_backAction->setShortcut(QKeySequence(QStringLiteral("Alt+Left")));
    connect(m_backAction, &QAction::triggered, this, &MainWindow::goBack);

    m_forwardAction = new QAction(QIcon::fromTheme(QStringLiteral("go-next")), tr("Forward"), this);
    m_forwardAction->setShortcut(QKeySequence(QStringLiteral("Alt+Right")));
    connect(m_forwardAction, &QAction::triggered, this, &MainWindow::goForward);

    m_upAction = new QAction(QIcon::fromTheme(QStringLiteral("go-up")), tr("Up"), this);
    m_upAction->setShortcut(QKeySequence(QStringLiteral("Alt+Up")));
    connect(m_upAction, &QAction::triggered, this, &MainWindow::goUp);

    m_homeAction = new QAction(QIcon::fromTheme(QStringLiteral("go-home")), tr("Home"), this);
    m_homeAction->setShortcut(QKeySequence(QStringLiteral("Alt+Home")));
    connect(m_homeAction, &QAction::triggered, this, &MainWindow::goHome);

    m_reloadAction = new QAction(QIcon::fromTheme(QStringLiteral("view-refresh")), tr("Reload"), this);
    m_reloadAction->setShortcut(QKeySequence::Refresh);
    connect(m_reloadAction, &QAction::triggered, this, &MainWindow::reload);

    m_showHiddenAction = new QAction(tr("Show Hidden Files"), this);
    m_showHiddenAction->setCheckable(true);
    m_showHiddenAction->setShortcut(QKeySequence(QStringLiteral("Ctrl+H")));
    connect(m_showHiddenAction, &QAction::toggled, this, &MainWindow::setShowHidden);

    m_newFolderAction = new QAction(QIcon::fromTheme(QStringLiteral("folder-new")), tr("New Folder..."), this);
    m_newFolderAction->setShortcut(QKeySequence(QStringLiteral("Ctrl+Shift+N")));
    connect(m_newFolderAction, &QAction::triggered, this, &MainWindow::newFolder);

    m_cutAction = new QAction(QIcon::fromTheme(QStringLiteral("edit-cut")), tr("Cut"), this);
    m_cutAction->setShortcut(QKeySequence::Cut);
    connect(m_cutAction, &QAction::triggered, this, &MainWindow::cutSelected);

    m_copyAction = new QAction(QIcon::fromTheme(QStringLiteral("edit-copy")), tr("Copy"), this);
    m_copyAction->setShortcut(QKeySequence::Copy);
    connect(m_copyAction, &QAction::triggered, this, &MainWindow::copySelected);

    m_pasteAction = new QAction(QIcon::fromTheme(QStringLiteral("edit-paste")), tr("Paste"), this);
    m_pasteAction->setShortcut(QKeySequence::Paste);
    connect(m_pasteAction, &QAction::triggered, this, &MainWindow::pasteClipboard);

    m_renameAction = new QAction(tr("Rename..."), this);
    m_renameAction->setShortcut(QKeySequence(QStringLiteral("F2")));
    connect(m_renameAction, &QAction::triggered, this, &MainWindow::renameSelected);

    m_trashAction = new QAction(QIcon::fromTheme(QStringLiteral("user-trash")), tr("Move to Trash"), this);
    m_trashAction->setShortcut(QKeySequence::Delete);
    connect(m_trashAction, &QAction::triggered, this, &MainWindow::trashSelected);

    // Actions must be on the window for their shortcuts to work everywhere.
    addAction(m_newFolderAction);
    addAction(m_cutAction);
    addAction(m_copyAction);
    addAction(m_pasteAction);
    addAction(m_renameAction);
    addAction(m_trashAction);
    addAction(m_backAction);
    addAction(m_forwardAction);
    addAction(m_upAction);
    addAction(m_homeAction);
    addAction(m_reloadAction);
}

void MainWindow::setupMenus()
{
    auto *fileMenu = menuBar()->addMenu(tr("&File"));
    auto *quitAction = fileMenu->addAction(tr("&Quit"));
    quitAction->setShortcut(QKeySequence::Quit);
    connect(quitAction, &QAction::triggered, this, &QWidget::close);

    auto *editMenu = menuBar()->addMenu(tr("&Edit"));
    editMenu->addAction(m_cutAction);
    editMenu->addAction(m_copyAction);
    editMenu->addAction(m_pasteAction);
    editMenu->addSeparator();
    editMenu->addAction(m_renameAction);
    editMenu->addAction(m_trashAction);
    editMenu->addSeparator();
    editMenu->addAction(m_newFolderAction);

    auto *viewMenu = menuBar()->addMenu(tr("&View"));
    viewMenu->addAction(m_showHiddenAction);
    viewMenu->addSeparator();
    viewMenu->addAction(m_reloadAction);

    auto *goMenu = menuBar()->addMenu(tr("&Go"));
    goMenu->addAction(m_backAction);
    goMenu->addAction(m_forwardAction);
    goMenu->addAction(m_upAction);
    goMenu->addAction(m_homeAction);

    auto *helpMenu = menuBar()->addMenu(tr("&Help"));
    auto *aboutAction = helpMenu->addAction(tr("&About NexusFM"));
    connect(aboutAction, &QAction::triggered, this, [this] {
        QMessageBox::about(this, tr("About NexusFM"),
                           tr("NexusFM is a lightweight file manager built with Rust and Qt 6 Widgets."));
    });
}

void MainWindow::setupToolBar()
{
    auto *toolBar = addToolBar(tr("Navigation"));
    toolBar->setObjectName(QStringLiteral("navigationToolBar"));
    toolBar->setMovable(false);
    toolBar->addAction(m_backAction);
    toolBar->addAction(m_forwardAction);
    toolBar->addAction(m_upAction);

    m_location = new QLineEdit(this);
    m_location->setPlaceholderText(tr("Location"));
    m_location->setClearButtonEnabled(true);
    connect(m_location, &QLineEdit::returnPressed, this, &MainWindow::onLocationEntered);
    toolBar->addSeparator();
    toolBar->addWidget(m_location);
}

void MainWindow::setupCentral()
{
    m_places = new QListWidget(this);
    m_places->setMinimumWidth(140);
    for (const Place &place : defaultPlaces()) {
        auto *item = new QListWidgetItem(QIcon::fromTheme(QStringLiteral("folder")), place.label, m_places);
        item->setData(Qt::UserRole, place.path);
        item->setToolTip(place.path);
    }
    connect(m_places, &QListWidget::itemActivated, this, &MainWindow::onPlaceActivated);
    connect(m_places, &QListWidget::itemClicked, this, &MainWindow::onPlaceActivated);

    m_model = new FileModel(this);
    m_view = new QTreeView(this);
    m_view->setModel(m_model);
    m_view->setRootIsDecorated(false);
    m_view->setAlternatingRowColors(true);
    m_view->setSelectionMode(QAbstractItemView::ExtendedSelection);
    m_view->setSelectionBehavior(QAbstractItemView::SelectRows);
    m_view->setSortingEnabled(false); // ordering comes from the Rust core
    m_view->setIndentation(0);
    m_view->header()->setSectionsClickable(true);
    m_view->header()->setStretchLastSection(true);
    m_view->header()->resizeSection(FileModel::NameColumn, 320);
    m_view->header()->resizeSection(FileModel::SizeColumn, 110);
    m_view->header()->resizeSection(FileModel::TypeColumn, 90);
    connect(m_view, &QTreeView::activated, this, &MainWindow::onItemActivated);
    connect(m_view->header(), &QHeaderView::sectionClicked, this, &MainWindow::onHeaderClicked);
    connect(m_view->selectionModel(), &QItemSelectionModel::selectionChanged, this, &MainWindow::updateEditActions);
    m_view->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(m_view, &QWidget::customContextMenuRequested, this, &MainWindow::showContextMenu);

    m_contextMenu = new QMenu(this);
    m_contextMenu->addAction(m_newFolderAction);
    m_contextMenu->addSeparator();
    m_contextMenu->addAction(m_cutAction);
    m_contextMenu->addAction(m_copyAction);
    m_contextMenu->addAction(m_pasteAction);
    m_contextMenu->addSeparator();
    m_contextMenu->addAction(m_renameAction);
    m_contextMenu->addAction(m_trashAction);

    auto *splitter = new QSplitter(Qt::Horizontal, this);
    splitter->addWidget(m_places);
    splitter->addWidget(m_view);
    splitter->setStretchFactor(1, 1);
    splitter->setCollapsible(0, true);
    splitter->setSizes({200, 760});
    setCentralWidget(splitter);

    updateSortIndicator();
}

void MainWindow::setupStatusBar()
{
    statusBar()->setSizeGripEnabled(true);
}

void MainWindow::navigateTo(const QString &path)
{
    const QString target = QDir::cleanPath(path);
    const QString previous = m_currentPath;
    if (loadDirectory(target) && !previous.isEmpty() && previous != m_currentPath) {
        m_backHistory.append(previous);
        m_forwardHistory.clear();
        updateNavigationActions();
    }
}

bool MainWindow::loadDirectory(const QString &path)
{
    const nfm::ReadResult result = nfm::readDirectory(path, m_settings);

    if (!result.ok) {
        m_model->setEntries({});
        m_location->setText(path);
        statusBar()->showMessage(result.error);
        return false;
    }

    m_currentPath = path;
    m_model->setEntries(result.entries);
    m_view->scrollToTop();

    m_location->setText(m_currentPath);
    const QString name = QDir(m_currentPath).dirName();
    setWindowTitle(tr("%1 - NexusFM").arg(name.isEmpty() ? QStringLiteral("/") : name));
    statusBar()->showMessage(tr("%n item(s)", nullptr, static_cast<int>(result.entries.size())));
    updateNavigationActions();
    return true;
}

void MainWindow::updateNavigationActions()
{
    updateEditActions();
    m_backAction->setEnabled(!m_backHistory.isEmpty());
    m_forwardAction->setEnabled(!m_forwardHistory.isEmpty());
    m_upAction->setEnabled(!QDir(m_currentPath).isRoot());
}

void MainWindow::updateSortIndicator()
{
    int column = FileModel::NameColumn;
    switch (m_settings.sortKey) {
    case nfm::SortKey::Name:
        column = FileModel::NameColumn;
        break;
    case nfm::SortKey::Size:
        column = FileModel::SizeColumn;
        break;
    case nfm::SortKey::Modified:
        column = FileModel::ModifiedColumn;
        break;
    }
    m_view->header()->setSortIndicator(column, m_settings.descending ? Qt::DescendingOrder : Qt::AscendingOrder);
}

void MainWindow::goBack()
{
    if (m_backHistory.isEmpty())
        return;
    const QString current = m_currentPath;
    const QString previous = m_backHistory.takeLast();
    if (loadDirectory(previous))
        m_forwardHistory.append(current);
    else
        m_backHistory.append(previous);
    updateNavigationActions();
}

void MainWindow::goForward()
{
    if (m_forwardHistory.isEmpty())
        return;
    const QString current = m_currentPath;
    const QString next = m_forwardHistory.takeLast();
    if (loadDirectory(next))
        m_backHistory.append(current);
    else
        m_forwardHistory.append(next);
    updateNavigationActions();
}

void MainWindow::goUp()
{
    QDir dir(m_currentPath);
    if (dir.isRoot())
        return;
    dir.cdUp();
    navigateTo(dir.absolutePath());
}

void MainWindow::goHome()
{
    navigateTo(QDir::homePath());
}

void MainWindow::reload()
{
    loadDirectory(m_currentPath);
}

void MainWindow::setShowHidden(bool show)
{
    m_settings.showHidden = show;
    reload();
}

void MainWindow::onItemActivated(const QModelIndex &index)
{
    const nfm::FileEntry *entry = m_model->entryAt(index.row());
    if (!entry)
        return;

    if (entry->isDirectory())
        navigateTo(entry->path);
    else
        QDesktopServices::openUrl(QUrl::fromLocalFile(entry->path));
}

void MainWindow::onPlaceActivated(QListWidgetItem *item)
{
    if (item)
        navigateTo(item->data(Qt::UserRole).toString());
}

void MainWindow::onLocationEntered()
{
    const QString path = m_location->text().trimmed();
    if (!path.isEmpty())
        navigateTo(path);
}

void MainWindow::onHeaderClicked(int column)
{
    nfm::SortKey key = nfm::SortKey::Name;
    switch (column) {
    case FileModel::SizeColumn:
        key = nfm::SortKey::Size;
        break;
    case FileModel::ModifiedColumn:
        key = nfm::SortKey::Modified;
        break;
    case FileModel::NameColumn:
    case FileModel::TypeColumn:
    default:
        key = nfm::SortKey::Name;
        break;
    }

    if (key == m_settings.sortKey) {
        m_settings.descending = !m_settings.descending;
    } else {
        m_settings.sortKey = key;
        m_settings.descending = false;
    }
    updateSortIndicator();
    reload();
}

QStringList MainWindow::selectedPaths() const
{
    QStringList paths;
    const QModelIndexList rows = m_view->selectionModel()->selectedRows();
    for (const QModelIndex &index : rows) {
        if (const nfm::FileEntry *entry = m_model->entryAt(index.row()))
            paths.append(entry->path);
    }
    return paths;
}

void MainWindow::reportErrors(const QString &title, const QStringList &errors)
{
    if (errors.isEmpty())
        return;
    QMessageBox::warning(this, title, errors.join(QLatin1Char('\n')));
}

void MainWindow::updateEditActions()
{
    const int selected = m_view ? m_view->selectionModel()->selectedRows().size() : 0;
    m_cutAction->setEnabled(selected > 0);
    m_copyAction->setEnabled(selected > 0);
    m_trashAction->setEnabled(selected > 0);
    m_renameAction->setEnabled(selected == 1);
    m_pasteAction->setEnabled(!m_clipboard.isEmpty());
    m_newFolderAction->setEnabled(!m_currentPath.isEmpty());
}

void MainWindow::newFolder()
{
    bool ok = false;
    const QString name = QInputDialog::getText(this, tr("New Folder"), tr("Folder name:"),
                                               QLineEdit::Normal, tr("New Folder"), &ok).trimmed();
    if (!ok)
        return;
    const QString error = nfm::ops::createFolder(m_currentPath, name);
    if (!error.isEmpty())
        QMessageBox::warning(this, tr("New Folder"), error);
    reload();
}

void MainWindow::renameSelected()
{
    const QStringList paths = selectedPaths();
    if (paths.size() != 1)
        return;

    const QString oldName = QFileInfo(paths.first()).fileName();
    bool ok = false;
    const QString newName = QInputDialog::getText(this, tr("Rename"), tr("New name:"),
                                                  QLineEdit::Normal, oldName, &ok).trimmed();
    if (!ok || newName == oldName)
        return;

    const QString error = nfm::ops::rename(paths.first(), newName);
    if (!error.isEmpty())
        QMessageBox::warning(this, tr("Rename"), error);
    reload();
}

void MainWindow::trashSelected()
{
    const QStringList paths = selectedPaths();
    if (paths.isEmpty())
        return;

    QStringList errors;
    for (const QString &path : paths) {
        const QString error = nfm::ops::moveToTrash(path);
        if (!error.isEmpty())
            errors.append(error);
    }
    reportErrors(tr("Move to Trash"), errors);
    reload();
}

void MainWindow::cutSelected()
{
    m_clipboard = selectedPaths();
    m_clipboardIsCut = true;
    updateEditActions();
}

void MainWindow::copySelected()
{
    m_clipboard = selectedPaths();
    m_clipboardIsCut = false;
    updateEditActions();
}

void MainWindow::pasteClipboard()
{
    if (m_clipboard.isEmpty() || m_currentPath.isEmpty())
        return;

    QStringList errors;
    QStringList remaining;
    for (const QString &path : std::as_const(m_clipboard)) {
        const QString error = m_clipboardIsCut
            ? nfm::ops::moveInto(path, m_currentPath)
            : nfm::ops::copyInto(path, m_currentPath);
        if (!error.isEmpty()) {
            errors.append(error);
            remaining.append(path);
        }
    }

    // A cut is one-shot: keep only the items that failed so they can be retried.
    if (m_clipboardIsCut)
        m_clipboard = remaining;
    reportErrors(tr("Paste"), errors);
    reload();
}

void MainWindow::showContextMenu(const QPoint &pos)
{
    const QModelIndex index = m_view->indexAt(pos);
    if (index.isValid() && !m_view->selectionModel()->isSelected(index))
        m_view->selectionModel()->select(index, QItemSelectionModel::ClearAndSelect | QItemSelectionModel::Rows);
    m_contextMenu->exec(m_view->viewport()->mapToGlobal(pos));
}
