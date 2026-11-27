// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

#include "MainWindow.h"

#include <QApplication>
#include <QDir>
#include <QIcon>

int main(int argc, char *argv[])
{
    QApplication app(argc, argv);
    QApplication::setApplicationName(QStringLiteral("NexusFM"));
    QApplication::setOrganizationName(QStringLiteral("NexusFM"));

    app.setWindowIcon(QIcon(QStringLiteral(":/icon.png")));

    const QString startPath = argc > 1 ? QString::fromLocal8Bit(argv[1]) : QDir::homePath();

    MainWindow window(startPath);
    window.show();
    return app.exec();
}
