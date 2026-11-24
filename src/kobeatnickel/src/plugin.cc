#include <QAction>
#include <QActionEvent>
#include <QApplication>
#include <QCoreApplication>
#include <QFile>
#include <QGuiApplication>
#include <QHBoxLayout>
#include <QImage>
#include <QLabel>
#include <QMenu>
#include <QPainter>
#include <QPainterPath>
#include <QPushButton>
#include <QScreen>
#include <QThread>
#include <QVBoxLayout>
#include <QSocketNotifier>
#include <QWidgetAction>

#include <cerrno>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#include <NickelHook.h>

#include "nickel.h"
#include "player_view.h"
#include "touch.h"
#include "plugin.h"

static char const kNoNavTabFlag[] = "/mnt/onboard/.adds/kobeat/no_nav_tab";
static char const kNoReaderMenuFlag[] = "/mnt/onboard/.adds/kobeat/no_reader_menu";
static char const kTitle[] = "Kobeat";
static char const kNavIcon[] = "/tmp/kobeat_nav.png";
static char const kNavIconActive[] = "/tmp/kobeat_nav_active.png";
static char const kNavButtonName[] = "kobeatNavButton";
static char const kInjectedProp[] = "kobeatInjected";
// Writing "open", "close" or "toggle" (one per line) here controls the player,
// e.g. from a NickelMenu cmd_spawn entry or a KFMon script.
static char const kTriggerFifo[] = "/tmp/kobeat-ui.fifo";
// Flag files that hide the built-in entry points.

static KobeatController *controller = nullptr;

static void bootstrapOnGuiThread() {
    KobeatController::bootstrap();
}

void KobeatController::bootstrap() {
    if (controller)
        return;
    QCoreApplication *app = QCoreApplication::instance();
    if (!app) {
        // Plugins can be loaded before Nickel creates its QApplication; Qt
        // runs pre-routines on the GUI thread from the application ctor.
        qAddPreRoutine(bootstrapOnGuiThread);
        nh_log("no QApplication yet, deferring startup");
        return;
    }
    controller = new KobeatController();
    if (controller->thread() != app->thread())
        controller->moveToThread(app->thread());
    // Don't touch widgets until the event loop runs on the GUI thread.
    QMetaObject::invokeMethod(controller, "start", Qt::QueuedConnection);
}

void KobeatController::start() {
    m_navTabEnabled = !QFile::exists(kNoNavTabFlag);
    m_readerMenuEnabled = !QFile::exists(kNoReaderMenuFlag);
    openTrigger();
    qApp->installEventFilter(this);
    // The nav bar may have been built before we were loaded.
    foreach (QWidget *widget, QApplication::allWidgets())
        if (widget->inherits("MainNavView"))
            injectNav(widget);
    nh_log("started (native dialog: %d, nav tab: %d, reader menu: %d, trigger: %d)", kobeat_nickel.hasNativeDialog(),
           m_navTabEnabled && kobeat_nickel.hasNavButton(), m_readerMenuEnabled && kobeat_nickel.hasMenuTextItem(),
           m_triggerFd >= 0);
}

void KobeatController::openTrigger() {
    struct stat st;
    if (lstat(kTriggerFifo, &st) == 0 && !S_ISFIFO(st.st_mode))
        unlink(kTriggerFifo); // e.g. a plain file left by `echo` while we weren't running
    if (mkfifo(kTriggerFifo, 0666) < 0 && errno != EEXIST) {
        nh_log("could not create %s: %m", kTriggerFifo);
        return;
    }
    // O_RDWR keeps a writer open ourselves, so the FIFO never reports EOF
    // after each external writer closes, and writers never block.
    m_triggerFd = open(kTriggerFifo, O_RDWR | O_NONBLOCK | O_CLOEXEC);
    if (m_triggerFd < 0) {
        nh_log("could not open %s: %m", kTriggerFifo);
        return;
    }
    QSocketNotifier *notifier = new QSocketNotifier(m_triggerFd, QSocketNotifier::Read, this);
    connect(notifier, SIGNAL(activated(int)), this, SLOT(onTrigger()));
}

void KobeatController::onTrigger() {
    char buf[256];
    ssize_t n;
    while ((n = read(m_triggerFd, buf, sizeof(buf))) > 0)
        m_triggerBuf.append(buf, n);
    int nl;
    while ((nl = m_triggerBuf.indexOf('\n')) >= 0) {
        QByteArray cmd = m_triggerBuf.left(nl).trimmed();
        m_triggerBuf.remove(0, nl + 1);
        if (cmd == "open")
            openPlayer();
        else if (cmd == "close")
            closePlayer();
        else if (cmd == "toggle")
            m_player ? closePlayer() : openPlayer();
        else if (!cmd.isEmpty())
            nh_log("unknown trigger command: %s", cmd.constData());
    }
    if (m_triggerBuf.size() > 1024)
        m_triggerBuf.clear();
}

bool KobeatController::eventFilter(QObject *obj, QEvent *event) {
    // This sees every event in Nickel, so filter by type before anything else.
    switch (event->type()) {
    case QEvent::Polish:
    case QEvent::Show:
        if (obj->inherits("MainNavView") && !obj->property(kInjectedProp).toBool())
            injectNav(static_cast<QWidget *>(obj));
        break;
    case QEvent::ActionAdded:
        if (QMenu *menu = qobject_cast<QMenu *>(obj))
            watchMenu(menu, static_cast<QActionEvent *>(event)->action());
        break;
    default:
        break;
    }
    return false;
}

// Draws a beamed pair of eighth notes, matching the stock icon's size.
static bool writeNavIcon(char const *path, bool active) {
    QSize size = QImage(":/images/home/main_nav_more.png").size();
    if (size.isEmpty())
        size = QSize(42, 42);
    QImage img(size, QImage::Format_ARGB32);
    img.fill(Qt::transparent);

    QPainter p(&img);
    p.setRenderHint(QPainter::Antialiasing);
    qreal w = size.width(), h = size.height();
    qreal stem = qMax<qreal>(2, w / (active ? 12 : 16));
    qreal headW = w * 0.30, headH = h * 0.22;
    QPointF left(w * 0.30, h * 0.76), right(w * 0.74, h * 0.68);
    qreal top = h * 0.12, beam = h * (active ? 0.16 : 0.12);

    p.setPen(Qt::NoPen);
    p.setBrush(Qt::black);
    QPainterPath beamPath;
    beamPath.moveTo(left.x() + headW / 2 - stem, top + h * 0.08);
    beamPath.lineTo(right.x() + headW / 2, top);
    beamPath.lineTo(right.x() + headW / 2, top + beam);
    beamPath.lineTo(left.x() + headW / 2 - stem, top + h * 0.08 + beam);
    beamPath.closeSubpath();
    p.drawPath(beamPath);
    p.drawRect(QRectF(left.x() + headW / 2 - stem, top + h * 0.08, stem, left.y() - top - h * 0.08));
    p.drawRect(QRectF(right.x() + headW / 2 - stem, top, stem, right.y() - top));

    foreach (QPointF c, QList<QPointF>() << left << right) {
        p.save();
        p.translate(c);
        p.rotate(-20);
        if (active) {
            p.drawEllipse(QRectF(-headW / 2, -headH / 2, headW, headH));
        } else {
            p.setBrush(Qt::NoBrush);
            p.setPen(QPen(Qt::black, stem));
            p.drawEllipse(QRectF(-headW / 2 + stem / 2, -headH / 2 + stem / 2, headW - stem, headH - stem));
        }
        p.restore();
    }
    p.end();
    return img.save(QString::fromLatin1(path), "PNG");
}

void KobeatController::injectNav(QWidget *nav) {
    nav->setProperty(kInjectedProp, true);
    m_nav = nav;
    QHBoxLayout *bar = nav->findChild<QHBoxLayout *>();
    if (!bar) {
        nh_log("no QHBoxLayout in MainNavView, not adding a tab");
        return;
    }

    // Remember which stock tab is active so it can be restored when the player
    // closes (4.46 has no MainNavView::getActiveNavButton). Nickel starts on
    // Home, the first tab; after that, track taps.
    for (int i = 0; i < bar->count(); i++) {
        QWidget *w = bar->itemAt(i)->widget();
        if (w && w->inherits("MainNavButton") && w->objectName() != kNavButtonName) {
            if (!m_lastTab)
                m_lastTab = w;
            connect(w, SIGNAL(tapped()), this, SLOT(onOtherNavTapped()));
        }
    }

    if (!m_navTabEnabled)
        return;
    if (!kobeat_nickel.hasNavButton()) {
        nh_log("MainNavButton symbols missing, not adding a tab");
        return;
    }

    QWidget *btn = static_cast<QWidget *>(kobeat_native_alloc());
    if (!btn)
        return;
    kobeat_nickel.MainNavButton_ctor(btn, nav);
    kobeat_nickel.MainNavButton_setText(btn, QString::fromLatin1(kTitle));
    if (kobeat_nickel.MainNavButton_setPixmap)
        kobeat_nickel.MainNavButton_setPixmap(btn, writeNavIcon(kNavIcon, false)
            ? QString::fromLatin1(kNavIcon) : QString(":/images/home/main_nav_more.png"));
    if (kobeat_nickel.MainNavButton_setActivePixmap)
        kobeat_nickel.MainNavButton_setActivePixmap(btn, writeNavIcon(kNavIconActive, true)
            ? QString::fromLatin1(kNavIconActive) : QString(":/images/home/main_nav_more_active.png"));
    btn->setObjectName(kNavButtonName);
    if (!connect(btn, SIGNAL(tapped()), this, SLOT(onNavTapped()))) {
        nh_log("could not connect MainNavButton::tapped, not adding a tab");
        delete btn;
        return;
    }
    m_navButton = btn;
    bar->addWidget(btn, 1);
    nh_log("added nav tab");
}

void KobeatController::watchMenu(QMenu *menu, QObject *action) {
    if (!m_readerMenuEnabled || !kobeat_nickel.hasMenuTextItem() || menu->property(kInjectedProp).toBool() || !menu->inherits("NickelTouchMenu"))
        return;
    QWidgetAction *wa = qobject_cast<QWidgetAction *>(action);
    if (!wa || !wa->defaultWidget())
        return;
    QObject *label = wa->defaultWidget()->findChild<QObject *>("textLabel");
    if (!label)
        return;
    // Same detection as NickelMenu: the reader's menu contains "Dictionary".
    if (label->property("text").toString() != QCoreApplication::translate("DictionaryActionProxy", "Dictionary"))
        return;
    menu->setProperty(kInjectedProp, true);
    connect(menu, SIGNAL(aboutToShow()), this, SLOT(onMenuAboutToShow()));
}

void KobeatController::onMenuAboutToShow() {
    QMenu *menu = qobject_cast<QMenu *>(sender());
    if (!menu || menu->property("kobeatItemAdded").toBool())
        return;
    menu->setProperty("kobeatItemAdded", true);

    QWidget *item = static_cast<QWidget *>(kobeat_native_alloc());
    if (!item)
        return;
    kobeat_nickel.MenuTextItem_ctor(item, menu, false, true);
    kobeat_nickel.MenuTextItem_setText(item, QString::fromLatin1(kTitle));
    kobeat_nickel.MenuTextItem_registerForTapGestures(item);

    QWidgetAction *action = new QWidgetAction(menu);
    action->setDefaultWidget(item);
    menu->addSeparator();
    menu->addAction(action);
    // MenuTextItem reports taps itself; forward them like Nickel's own items.
    connect(item, SIGNAL(tapped(bool)), action, SIGNAL(triggered()));
    connect(action, SIGNAL(triggered()), menu, SLOT(hide()));
    connect(action, SIGNAL(triggered()), this, SLOT(onMenuItemTriggered()));
}

void KobeatController::onMenuItemTriggered() {
    // Let the menu finish closing before covering the screen.
    QMetaObject::invokeMethod(this, "openPlayer", Qt::QueuedConnection);
}

void KobeatController::onNavTapped() {
    if (m_player) {
        m_player->raise();
        return;
    }
    openPlayer();
}

void KobeatController::onOtherNavTapped() {
    QWidget *tab = qobject_cast<QWidget *>(sender());
    m_lastTab = tab;
    if (m_player) {
        // Nickel switches the view underneath us; get out of the way and
        // let the tapped tab take over the highlight.
        m_restoreTab = tab;
        closePlayer();
    }
}

void KobeatController::setNavActive(bool active) {
    if (!kobeat_nickel.MainNavButton_setActive || !m_navButton)
        return;
    if (active) {
        m_restoreTab = m_lastTab;
        if (m_restoreTab)
            kobeat_nickel.MainNavButton_setActive(m_restoreTab, false);
        kobeat_nickel.MainNavButton_setActive(m_navButton, true);
    } else {
        kobeat_nickel.MainNavButton_setActive(m_navButton, false);
        if (m_restoreTab)
            kobeat_nickel.MainNavButton_setActive(m_restoreTab, true);
        m_restoreTab = nullptr;
    }
}

QWidget *KobeatController::createWindow() {
    // Cover the whole screen, bottom nav bar included: the player has its own
    // back button, and the extra height goes to the music browser.
    QRect area = QGuiApplication::primaryScreen()->availableGeometry();

    KobeatPlayerView *view = new KobeatPlayerView();
    QWidget *window = nullptr;

    if (kobeat_nickel.hasNativeDialog() && (window = static_cast<QWidget *>(kobeat_native_alloc()))) {
        kobeat_nickel.N3Dialog_ctor(window, nullptr, false);
        kobeat_nickel.N3Dialog_setTitle(window, QString::fromLatin1(kTitle));
        kobeat_nickel.N3Dialog_enableFullViewMode(window);
        kobeat_nickel.N3Dialog_enableBackButton(window, true);
        kobeat_nickel.N3Dialog_disableCloseButton(window);
        kobeat_nickel.N3Dialog_setContent(window, view);
        connect(window, SIGNAL(backTapped()), this, SLOT(closePlayer()));
    } else {
        nh_log("N3Dialog unavailable, using a plain window");
        window = new QWidget(nullptr, Qt::Window | Qt::FramelessWindowHint);
        window->setStyleSheet("background: white;");
        QVBoxLayout *layout = new QVBoxLayout(window);
        layout->setContentsMargins(0, 0, 0, 0);
        QHBoxLayout *header = new QHBoxLayout();
        QLabel *title = new QLabel(QString::fromLatin1(kTitle), window);
        title->setStyleSheet(QString("color: black; font-weight: bold; font-size: %1px; padding: 12px;").arg(area.height() / 36));
        QPushButton *close = new QPushButton("Close", window);
        close->setStyleSheet(QString("color: black; background: white; border: 2px solid black; font-size: %1px; padding: 8px 24px;")
                                 .arg(area.height() / 48));
        connect(close, SIGNAL(clicked()), this, SLOT(closePlayer()));
        kobeat_make_tappable(close);
        header->addWidget(title, 1);
        header->addWidget(close);
        layout->addLayout(header);
        layout->addWidget(view, 1);
    }

    window->setAttribute(Qt::WA_DeleteOnClose);
    window->setGeometry(area);
    return window;
}

void KobeatController::openPlayer() {
    if (m_player) {
        m_player->raise();
        return;
    }
    m_player = createWindow();
    connect(m_player, SIGNAL(destroyed()), this, SLOT(onPlayerDestroyed()));
    setNavActive(true);
    m_player->show();
}

void KobeatController::closePlayer() {
    // Deferred so we never delete the dialog from inside its own signal.
    if (m_player)
        QMetaObject::invokeMethod(m_player, "close", Qt::QueuedConnection);
}

void KobeatController::onPlayerDestroyed() {
    setNavActive(false);
}

static int kobeat_init() {
    KobeatController::bootstrap();
    return 0;
}

static struct nh_info KobeatInfo = {
    .name = "Kobeat",
    .desc = "Native music player UI for the kobeat daemon",
    .uninstall_flag = "/mnt/onboard/.adds/kobeat/uninstall",
    .uninstall_xflag = nullptr,
    .failsafe_delay = 2,
};

static struct nh_hook KobeatHook[] = {
    {0},
};

#define KOBEAT_SYM(sym, field, what) \
    {.name = sym, .out = nh_symoutptr(kobeat_nickel.field), .desc = what, .optional = true}

static struct nh_dlsym KobeatDlsym[] = {
    KOBEAT_SYM("_ZN8N3DialogC1EP7QWidgetb", N3Dialog_ctor, "player dialog"),
    KOBEAT_SYM("_ZN8N3Dialog8setTitleERK7QString", N3Dialog_setTitle, "player dialog"),
    KOBEAT_SYM("_ZN8N3Dialog18enableFullViewModeEv", N3Dialog_enableFullViewMode, "player dialog"),
    KOBEAT_SYM("_ZN8N3Dialog16enableBackButtonEb", N3Dialog_enableBackButton, "player dialog"),
    KOBEAT_SYM("_ZN8N3Dialog18disableCloseButtonEv", N3Dialog_disableCloseButton, "player dialog"),
    KOBEAT_SYM("_ZN8N3Dialog10setContentEP7QWidget", N3Dialog_setContent, "player dialog"),
    KOBEAT_SYM("_ZN13MainNavButtonC1EP7QWidget", MainNavButton_ctor, "nav tab"),
    KOBEAT_SYM("_ZN13MainNavButton7setTextERK7QString", MainNavButton_setText, "nav tab"),
    KOBEAT_SYM("_ZN13MainNavButton9setPixmapERK7QString", MainNavButton_setPixmap, "nav tab"),
    KOBEAT_SYM("_ZN13MainNavButton15setActivePixmapERK7QString", MainNavButton_setActivePixmap, "nav tab"),
    KOBEAT_SYM("_ZN13MainNavButton9setActiveEb", MainNavButton_setActive, "nav tab"),
    KOBEAT_SYM("_ZN12MenuTextItemC1EP7QWidgetbb", MenuTextItem_ctor, "reader menu item"),
    KOBEAT_SYM("_ZN12MenuTextItem7setTextERK7QString", MenuTextItem_setText, "reader menu item"),
    KOBEAT_SYM("_ZN12MenuTextItem22registerForTapGesturesEv", MenuTextItem_registerForTapGestures, "reader menu item"),
    {0},
};

NickelHook(
    .init = &kobeat_init,
    .info = &KobeatInfo,
    .hook = KobeatHook,
    .dlsym = KobeatDlsym,
)
