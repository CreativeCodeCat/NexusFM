// Copyright (C) 2026 NexusFM
//
// This file is part of NexusFM, licensed under the GNU General Public License
// version 3. See the LICENSE file for details.

// Client-side title bar for the frameless main window: menu bar on the left,
// draggable space and minimize / maximize / close buttons on the right.
#pragma once

#include <QWidget>

class QMenuBar;
class QToolButton;

class TitleBar : public QWidget
{
    Q_OBJECT

public:
    TitleBar(QMenuBar *menuBar, QWidget *window);

    void updateMaximizeIcon(bool maximized);

protected:
    void mousePressEvent(QMouseEvent *event) override;
    void mouseDoubleClickEvent(QMouseEvent *event) override;

private:
    void toggleMaximized();

    QWidget *m_window = nullptr;
    QToolButton *m_maximizeButton = nullptr;
};
