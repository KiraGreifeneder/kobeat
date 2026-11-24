#pragma once

#include <QByteArray>
#include <QObject>
#include <QPointer>
#include <QWidget>

class QMenu;

// Lives on the GUI thread for the lifetime of Nickel. Watches widget creation
// through an application-wide event filter (no symbol hooks, so it cannot
// conflict with NickelMenu, Nickelite or other NickelHook mods) and adds:
//   * a "Kobeat" tab to the home screen's bottom navigation bar, and
//   * a "Kobeat" item to the reader's overflow menu,
// both of which open the full-screen player. Either can be turned off with a
// flag file, and the player can also be opened through a FIFO so other
// launchers (NickelMenu, KFMon, scripts) can drive it.
class KobeatController : public QObject {
    Q_OBJECT

public:
    static void bootstrap();

    bool eventFilter(QObject *obj, QEvent *event) override;

public slots:
    void start();
    void openPlayer();

private slots:
    void onNavTapped();
    void onOtherNavTapped();
    void onMenuAboutToShow();
    void onMenuItemTriggered();
    void closePlayer();
    void onPlayerDestroyed();
    void onTrigger();

private:
    KobeatController() : m_navTabEnabled(true), m_readerMenuEnabled(true), m_triggerFd(-1) {}

    void injectNav(QWidget *nav);
    void watchMenu(QMenu *menu, QObject *action);
    QWidget *createWindow();
    void setNavActive(bool active);
    void openTrigger();

    bool m_navTabEnabled;
    bool m_readerMenuEnabled;
    int m_triggerFd;
    QByteArray m_triggerBuf;

    QPointer<QWidget> m_player;
    QPointer<QWidget> m_nav;
    QPointer<QWidget> m_navButton;
    QPointer<QWidget> m_lastTab; // last stock tab the user tapped
    QPointer<QWidget> m_restoreTab;
};
