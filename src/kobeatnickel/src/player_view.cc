#include <QCheckBox>
#include <QDir>
#include <QFileInfo>
#include <QFrame>
#include <QGuiApplication>
#include <QHBoxLayout>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QMouseEvent>
#include <QPainter>
#include <QPushButton>
#include <QScreen>
#include <QTimer>
#include <QVBoxLayout>

#include "client.h"
#include "daemon.h"
#include "player_view.h"
#include "touch.h"

// Keep in sync with MUSIC_DIR in src/config.rs.
static char const kMusicDir[] = "/mnt/onboard/music";
// Upper bound; the rows actually shown depend on the available height.
static int const kMaxRows = 20;
// Matches the depth the web UI scans when building a queue.
static int const kQueueDepth = 3;

static QString formatTime(double secs) {
    if (secs < 0)
        secs = 0;
    qulonglong s = static_cast<qulonglong>(secs);
    if (s >= 3600)
        return QString("%1:%2:%3").arg(s / 3600).arg((s / 60) % 60, 2, 10, QChar('0')).arg(s % 60, 2, 10, QChar('0'));
    return QString("%1:%2").arg(s / 60).arg(s % 60, 2, 10, QChar('0'));
}

KobeatSeekBar::KobeatSeekBar(QWidget *parent) : QWidget(parent), m_fraction(0) {
    setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
}

void KobeatSeekBar::setFraction(double fraction) {
    if (fraction < 0)
        fraction = 0;
    if (fraction > 1)
        fraction = 1;
    // Only repaint when the filled width actually changes; every repaint costs
    // an e-ink refresh.
    int inner = width() - 4;
    if (static_cast<int>(m_fraction * inner) != static_cast<int>(fraction * inner)) {
        m_fraction = fraction;
        update();
    } else {
        m_fraction = fraction;
    }
}

void KobeatSeekBar::paintEvent(QPaintEvent *) {
    QPainter p(this);
    QRect r = rect().adjusted(1, height() / 4, -1, -height() / 4);
    p.fillRect(r, Qt::white);
    p.setPen(QPen(Qt::black, 2));
    p.drawRect(r);
    QRect fill = r.adjusted(2, 2, -2, -2);
    fill.setWidth(static_cast<int>(fill.width() * m_fraction));
    if (fill.width() > 0)
        p.fillRect(fill, Qt::black);
}

void KobeatSeekBar::tapAt(QPoint const &pos) {
    if (width() > 0 && rect().contains(pos))
        emit seekRequested(static_cast<double>(pos.x()) / width());
}

void KobeatSeekBar::mouseReleaseEvent(QMouseEvent *event) {
    tapAt(event->pos());
}

static QPushButton *makeButton(QString const &text, QWidget *parent) {
    QPushButton *b = new QPushButton(text, parent);
    b->setFocusPolicy(Qt::NoFocus);
    b->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Preferred);
    return b;
}

KobeatPlayerView::KobeatPlayerView(QWidget *parent)
    : QWidget(parent), m_stateInFlight(0), m_connectionError(true), m_daemonState(DaemonUnknown), m_daemonBusy(false),
      m_duration(0), m_rowsPerPage(1), m_page(0) {
    setObjectName("kobeatPlayerView");
    setAttribute(Qt::WA_StyledBackground, true);

    int screenH = 1680;
    if (QScreen *screen = QGuiApplication::primaryScreen())
        screenH = qMax(screen->geometry().height(), screen->geometry().width());
    int font = qMax(16, screenH / 48);
    int big = qMax(20, screenH / 36);
    int button = qMax(44, screenH / 24);
    int row = qMax(40, screenH / 24);
    m_rowHeight = row;

    setStyleSheet(QString(
        "#kobeatPlayerView { background: white; }"
        "#kobeatPlayerView QLabel, #kobeatPlayerView QCheckBox, #kobeatPlayerView QPushButton {"
        "  color: black; font-size: %1px; }"
        "#kobeatPlayerView #kobeatTitle { font-size: %2px; font-weight: bold; }"
        "#kobeatPlayerView QPushButton {"
        "  background: white; border: 2px solid black; border-radius: 6px; padding: 4px 12px;"
        "  min-height: %3px; }"
        "#kobeatPlayerView QPushButton:pressed { background: black; color: white; }"
        "#kobeatPlayerView QPushButton:disabled { color: #aaaaaa; border-color: #aaaaaa; }"
        "#kobeatPlayerView QPushButton[kobeatRow=\"true\"] {"
        "  border: none; border-bottom: 1px solid #888888; border-radius: 0; text-align: left;"
        "  min-height: 0px; }"
        "#kobeatPlayerView QCheckBox::indicator { width: %1px; height: %1px; }")
        .arg(font).arg(big).arg(button));

    QVBoxLayout *root = new QVBoxLayout(this);
    int margin = screenH / 60;
    root->setContentsMargins(margin, margin, margin, margin);
    root->setSpacing(margin / 2);

    m_status = new QLabel("Connecting to kobeatd...", this);
    m_status->setWordWrap(true);
    m_daemonButton = makeButton("Stop kobeatd", this);
    m_daemonButton->setSizePolicy(QSizePolicy::Fixed, QSizePolicy::Preferred);
    m_daemonButton->setEnabled(false);
    connect(m_daemonButton, SIGNAL(clicked()), this, SLOT(onDaemonButton()));
    QHBoxLayout *statusRow = new QHBoxLayout();
    statusRow->addWidget(m_status, 1);
    statusRow->addWidget(m_daemonButton);
    root->addLayout(statusRow);

    m_title = new QLabel("Nothing playing", this);
    m_title->setObjectName("kobeatTitle");
    m_title->setWordWrap(true);
    m_title->setMinimumHeight(big + 8);
    m_title->setAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    root->addWidget(m_title);

    m_seek = new KobeatSeekBar(this);
    m_seek->setFixedHeight(row);
    connect(m_seek, SIGNAL(seekRequested(double)), this, SLOT(onSeek(double)));
    root->addWidget(m_seek);

    m_time = new QLabel(this);
    m_time->setAlignment(Qt::AlignRight);
    root->addWidget(m_time);

    QHBoxLayout *transport = new QHBoxLayout();
    QPushButton *prev = makeButton("Previous", this);
    m_playPause = makeButton("Play", this);
    QPushButton *next = makeButton("Next", this);
    connect(prev, SIGNAL(clicked()), this, SLOT(onPrevious()));
    connect(m_playPause, SIGNAL(clicked()), this, SLOT(onPlayPause()));
    connect(next, SIGNAL(clicked()), this, SLOT(onNext()));
    transport->addWidget(prev);
    transport->addWidget(m_playPause);
    transport->addWidget(next);
    root->addLayout(transport);

    QFrame *line = new QFrame(this);
    line->setFrameShape(QFrame::HLine);
    line->setStyleSheet("color: black;");
    root->addWidget(line);

    QHBoxLayout *dirRow = new QHBoxLayout();
    m_up = makeButton("Up", this);
    m_up->setSizePolicy(QSizePolicy::Fixed, QSizePolicy::Preferred);
    connect(m_up, SIGNAL(clicked()), this, SLOT(onUp()));
    m_dirLabel = new QLabel(this);
    m_dirLabel->setSizePolicy(QSizePolicy::Ignored, QSizePolicy::Preferred);
    dirRow->addWidget(m_up);
    dirRow->addWidget(m_dirLabel, 1);
    root->addLayout(dirRow);

    m_list = new QWidget(this);
    m_list->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
    m_list->setMinimumHeight(row);
    m_list->installEventFilter(this);
    for (int i = 0; i < kMaxRows; i++) {
        QPushButton *b = makeButton(QString(), m_list);
        b->setProperty("kobeatRow", true);
        b->hide();
        connect(b, SIGNAL(clicked()), this, SLOT(onRowTapped()));
        m_rows.append(b);
    }
    root->addWidget(m_list, 1);

    QHBoxLayout *paging = new QHBoxLayout();
    m_prevPage = makeButton("Prev page", this);
    m_pageLabel = new QLabel(this);
    m_pageLabel->setAlignment(Qt::AlignCenter);
    m_nextPage = makeButton("Next page", this);
    connect(m_prevPage, SIGNAL(clicked()), this, SLOT(onPrevPage()));
    connect(m_nextPage, SIGNAL(clicked()), this, SLOT(onNextPage()));
    paging->addWidget(m_prevPage);
    paging->addWidget(m_pageLabel);
    paging->addWidget(m_nextPage);
    root->addLayout(paging);

    QHBoxLayout *queue = new QHBoxLayout();
    m_recursive = new QCheckBox("Include subfolders", this);
    m_recursive->setFocusPolicy(Qt::NoFocus);
    QPushButton *playFolder = makeButton("Play this folder", this);
    connect(playFolder, SIGNAL(clicked()), this, SLOT(onPlayFolder()));
    queue->addWidget(m_recursive);
    queue->addWidget(playFolder);
    root->addLayout(queue);

    m_poll = new QTimer(this);
    m_poll->setInterval(1000);
    connect(m_poll, SIGNAL(timeout()), this, SLOT(poll()));

    connect(KobeatClient::instance(), SIGNAL(replied(int, QByteArray, QString)),
            this, SLOT(onReplied(int, QByteArray, QString)));

    kobeat_make_tappable(this);
    // Same as kobeatd: give a fresh install somewhere to copy music into.
    QDir().mkpath(QString::fromLatin1(kMusicDir));
    openDir(QString::fromLatin1(kMusicDir));
}

bool KobeatPlayerView::eventFilter(QObject *obj, QEvent *event) {
    if (obj == m_list && event->type() == QEvent::Resize)
        layoutRows();
    return QWidget::eventFilter(obj, event);
}

void KobeatPlayerView::layoutRows() {
    int n = qBound(1, m_list->height() / m_rowHeight, kMaxRows);
    int first = m_page * m_rowsPerPage;
    m_rowsPerPage = n;
    m_page = first / n;
    for (int i = 0; i < m_rows.size(); i++) {
        QPushButton *b = m_rows.at(i);
        if (i < n) {
            b->setGeometry(0, i * m_rowHeight, m_list->width(), m_rowHeight);
            b->show();
        } else {
            b->hide();
        }
    }
    renderPage();
}

void KobeatPlayerView::showEvent(QShowEvent *event) {
    QWidget::showEvent(event);
    poll();
    m_poll->start();
}

void KobeatPlayerView::hideEvent(QHideEvent *event) {
    m_poll->stop();
    QWidget::hideEvent(event);
}

void KobeatPlayerView::poll() {
    if (m_stateInFlight > 0)
        return;
    m_stateInFlight++;
    emit KobeatClient::instance()->send(KobeatClient::State, KobeatClient::unitCommand("GetPlayerState"));
}

void KobeatPlayerView::command(QByteArray const &body) {
    emit KobeatClient::instance()->send(KobeatClient::Command, body);
}

void KobeatPlayerView::setStatus(QString const &text) {
    m_status->setText(text);
}

void KobeatPlayerView::onReplied(int kind, QByteArray response, QString error) {
    if (kind == KobeatClient::State && m_stateInFlight > 0)
        m_stateInFlight--;

    if (!error.isEmpty()) {
        m_connectionError = true;
        if (error == "kobeatd is not running") {
            setDaemonState(DaemonStopped);
            // Expected while kobeatd is still starting up.
            if (m_daemonBusy)
                return;
        }
        setStatus(error);
        return;
    }
    if (kind != KobeatClient::Shutdown)
        setDaemonState(DaemonRunning);

    if (kind == KobeatClient::State) {
        applyState(response);
        return;
    }

    QJsonObject obj = QJsonDocument::fromJson(response).object();
    if (obj.contains("Error"))
        setStatus("Error: " + obj.value("Error").toString());
    else if (obj.contains("Ok"))
        setStatus(obj.value("Ok").toString());
    // Reflect the command's effect without waiting for the next tick.
    poll();
}

void KobeatPlayerView::setDaemonState(int state) {
    if (state == DaemonStopped && m_daemonState != DaemonStopped) {
        m_playback.clear();
        m_duration = 0;
        m_title->setText("Nothing playing");
        m_time->clear();
        m_seek->setFraction(0);
        m_playPause->setText("Play");
    }
    m_daemonState = state;
    updateDaemonButton();
}

void KobeatPlayerView::updateDaemonButton() {
    m_daemonButton->setText(m_daemonState == DaemonStopped ? "Start kobeatd" : "Stop kobeatd");
    m_daemonButton->setEnabled(m_daemonState != DaemonUnknown && !m_daemonBusy);
}

void KobeatPlayerView::onDaemonButton() {
    if (m_daemonBusy)
        return;
    if (m_daemonState == DaemonRunning) {
        setStatus("Stopping kobeatd...");
        emit KobeatClient::instance()->send(KobeatClient::Shutdown, KobeatClient::unitCommand("Shutdown"));
    } else if (m_daemonState == DaemonStopped) {
        QString err = kobeat_start_daemon();
        if (!err.isEmpty()) {
            setStatus(err);
            return;
        }
        setStatus("Starting kobeatd...");
    } else {
        return;
    }
    m_daemonBusy = true;
    updateDaemonButton();
    QTimer::singleShot(3000, this, SLOT(onDaemonSettled()));
}

void KobeatPlayerView::onDaemonSettled() {
    m_daemonBusy = false;
    updateDaemonButton();
    poll();
}

void KobeatPlayerView::applyState(QByteArray const &response) {
    QJsonObject obj = QJsonDocument::fromJson(response).object();
    if (obj.contains("Error")) {
        setStatus("Error: " + obj.value("Error").toString());
        return;
    }
    if (!obj.value("State").isObject()) {
        setStatus("Unexpected response from kobeatd");
        return;
    }
    QJsonObject state = obj.value("State").toObject();

    m_playback = state.value("playback_state").toString();
    double position = state.value("base_position_secs").toDouble();
    m_playPause->setText(m_playback == "Playing" ? "Pause" : "Play");

    QJsonValue track = state.value("current_track");
    if (track.isObject()) {
        QJsonObject t = track.toObject();
        QString title = t.value("title").toString();
        if (title.isEmpty())
            title = QFileInfo(t.value("path").toString()).completeBaseName();
        m_title->setText(title);
        m_duration = t.value("duration_secs").toDouble();
    } else {
        m_title->setText("Nothing playing");
        m_duration = 0;
        position = 0;
    }

    // A good reply means any earlier connection error is stale.
    if (m_connectionError) {
        m_connectionError = false;
        setStatus(QString());
    }

    m_seek->setFraction(m_duration > 0 ? position / m_duration : 0);
    QString time = m_playback;
    if (m_duration > 0)
        time += "   " + formatTime(position) + " / " + formatTime(m_duration);
    m_time->setText(time);
}

void KobeatPlayerView::onPlayPause() {
    if (m_playback == "Playing")
        command(KobeatClient::unitCommand("Pause"));
    else if (m_playback == "Paused")
        command(KobeatClient::unitCommand("Resume"));
    else
        command(KobeatClient::unitCommand("PlayNext"));
}

void KobeatPlayerView::onPrevious() {
    command(KobeatClient::unitCommand("PlayPrevious"));
}

void KobeatPlayerView::onNext() {
    command(KobeatClient::unitCommand("PlayNext"));
}

void KobeatPlayerView::onSeek(double fraction) {
    if (m_duration <= 0)
        return;
    command(KobeatClient::seek(static_cast<qulonglong>(fraction * m_duration)));
}

void KobeatPlayerView::openDir(QString const &dir) {
    QDir d(dir);
    m_dir = d.absolutePath();
    m_entries.clear();
    if (!d.exists()) {
        setStatus(QString("%1 does not exist").arg(m_dir));
    } else {
        foreach (QString name, d.entryList(QDir::Dirs | QDir::NoDotAndDotDot, QDir::Name))
            m_entries.append(name + '/');
        m_entries.append(d.entryList(QDir::Files, QDir::Name));
    }
    m_page = 0;

    QString root = QDir(QString::fromLatin1(kMusicDir)).absolutePath();
    m_up->setEnabled(m_dir != root && m_dir.startsWith(root));
    QString rel = m_dir.mid(root.length());
    m_dirLabel->setText(rel.isEmpty() ? QString("Music") : "Music" + rel);
    renderPage();
}

void KobeatPlayerView::renderPage() {
    int pages = qMax(1, (m_entries.size() + m_rowsPerPage - 1) / m_rowsPerPage);
    m_page = qBound(0, m_page, pages - 1);
    for (int i = 0; i < m_rows.size(); i++) {
        int idx = m_page * m_rowsPerPage + i;
        QPushButton *b = m_rows.at(i);
        if (idx < m_entries.size()) {
            QString entry = m_entries.at(idx);
            b->setText(entry.endsWith('/') ? entry : entry);
            b->setProperty("kobeatEntry", entry);
            b->setEnabled(true);
        } else {
            b->setText(idx == 0 ? QString("(empty folder)") : QString());
            b->setProperty("kobeatEntry", QVariant());
            b->setEnabled(false);
        }
    }
    m_prevPage->setEnabled(m_page > 0);
    m_nextPage->setEnabled(m_page < pages - 1);
    m_pageLabel->setText(QString("%1 / %2").arg(m_page + 1).arg(pages));
}

void KobeatPlayerView::onRowTapped() {
    QPushButton *b = qobject_cast<QPushButton *>(sender());
    if (!b)
        return;
    QString entry = b->property("kobeatEntry").toString();
    if (entry.isEmpty())
        return;
    if (entry.endsWith('/'))
        openDir(m_dir + '/' + entry.left(entry.size() - 1));
    else
        command(KobeatClient::loadAndPlay(m_dir + '/' + entry));
}

void KobeatPlayerView::onUp() {
    openDir(QFileInfo(m_dir).path());
}

void KobeatPlayerView::onPrevPage() {
    m_page--;
    renderPage();
}

void KobeatPlayerView::onNextPage() {
    m_page++;
    renderPage();
}

QStringList KobeatPlayerView::collectFiles(QString const &dir, bool recursive, int depth) const {
    QDir d(dir);
    QStringList files;
    foreach (QString name, d.entryList(QDir::Files, QDir::Name))
        files.append(d.absoluteFilePath(name));
    if (recursive && depth > 0)
        foreach (QString name, d.entryList(QDir::Dirs | QDir::NoDotAndDotDot, QDir::Name))
            files.append(collectFiles(d.absoluteFilePath(name), true, depth - 1));
    return files;
}

void KobeatPlayerView::onPlayFolder() {
    QStringList files = collectFiles(m_dir, m_recursive->isChecked(), kQueueDepth);
    if (files.isEmpty()) {
        setStatus("No files in this folder");
        return;
    }
    // The daemon doesn't start playback when the queue changes, and the IPC
    // worker handles requests in order, so this plays the first queued file.
    command(KobeatClient::setQueue(files));
    command(KobeatClient::unitCommand("PlayNext"));
}
