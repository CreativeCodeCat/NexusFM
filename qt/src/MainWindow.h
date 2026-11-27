#pragma once

#include "Core.h"

#include <QMainWindow>
#include <QString>
#include <QVector>

class FileModel;
class QAction;
class QLineEdit;
class QListWidget;
class QListWidgetItem;
class QTreeView;

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    explicit MainWindow(const QString &startPath, QWidget *parent = nullptr);

    void navigateTo(const QString &path);

private slots:
    void goBack();
    void goForward();
    void goUp();
    void goHome();
    void reload();
    void setShowHidden(bool show);
    void onItemActivated(const QModelIndex &index);
    void onPlaceActivated(QListWidgetItem *item);
    void onLocationEntered();
    void onHeaderClicked(int column);

private:
    void setupActions();
    void setupMenus();
    void setupToolBar();
    void setupCentral();
    void setupStatusBar();
    bool loadDirectory(const QString &path);
    void updateNavigationActions();
    void updateSortIndicator();

    nfm::Settings m_settings;
    QString m_currentPath;
    QVector<QString> m_backHistory;
    QVector<QString> m_forwardHistory;

    FileModel *m_model = nullptr;
    QTreeView *m_view = nullptr;
    QListWidget *m_places = nullptr;
    QLineEdit *m_location = nullptr;

    QAction *m_backAction = nullptr;
    QAction *m_forwardAction = nullptr;
    QAction *m_upAction = nullptr;
    QAction *m_homeAction = nullptr;
    QAction *m_reloadAction = nullptr;
    QAction *m_showHiddenAction = nullptr;
};
