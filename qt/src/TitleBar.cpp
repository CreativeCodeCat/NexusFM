#include "TitleBar.h"

#include <QApplication>
#include <QHBoxLayout>
#include <QMenuBar>
#include <QMouseEvent>
#include <QStyle>
#include <QToolButton>
#include <QWidget>
#include <QWindow>

namespace {

QToolButton *makeButton(QWidget *parent, QStyle::StandardPixmap icon, const QString &tooltip)
{
    auto *button = new QToolButton(parent);
    button->setIcon(parent->style()->standardIcon(icon));
    button->setToolTip(tooltip);
    button->setAutoRaise(true);
    button->setFocusPolicy(Qt::NoFocus);
    return button;
}

} // namespace

TitleBar::TitleBar(QMenuBar *menuBar, QWidget *window)
    : QWidget(window)
    , m_window(window)
{
    setObjectName(QStringLiteral("titleBar"));

    auto *layout = new QHBoxLayout(this);
    layout->setContentsMargins(4, 0, 0, 0);
    layout->setSpacing(0);

    layout->addWidget(menuBar, 0, Qt::AlignVCenter);
    layout->addStretch(1); // empty space here drags the window

    auto *minimize = makeButton(this, QStyle::SP_TitleBarMinButton, tr("Minimize"));
    m_maximizeButton = makeButton(this, QStyle::SP_TitleBarMaxButton, tr("Maximize"));
    auto *close = makeButton(this, QStyle::SP_TitleBarCloseButton, tr("Close"));

    connect(minimize, &QToolButton::clicked, m_window, &QWidget::showMinimized);
    connect(m_maximizeButton, &QToolButton::clicked, this, &TitleBar::toggleMaximized);
    connect(close, &QToolButton::clicked, m_window, &QWidget::close);

    layout->addWidget(minimize);
    layout->addWidget(m_maximizeButton);
    layout->addWidget(close);
}

void TitleBar::updateMaximizeIcon(bool maximized)
{
    m_maximizeButton->setIcon(style()->standardIcon(maximized ? QStyle::SP_TitleBarNormalButton
                                                              : QStyle::SP_TitleBarMaxButton));
    m_maximizeButton->setToolTip(maximized ? tr("Restore") : tr("Maximize"));
}

void TitleBar::toggleMaximized()
{
    if (m_window->isMaximized())
        m_window->showNormal();
    else
        m_window->showMaximized();
}

void TitleBar::mousePressEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton && childAt(event->position().toPoint()) == nullptr) {
        if (QWindow *handle = m_window->windowHandle())
            handle->startSystemMove();
        event->accept();
        return;
    }
    QWidget::mousePressEvent(event);
}

void TitleBar::mouseDoubleClickEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton && childAt(event->position().toPoint()) == nullptr) {
        toggleMaximized();
        event->accept();
        return;
    }
    QWidget::mouseDoubleClickEvent(event);
}
