#include <cerrno>
#include <cstring>

#include <sys/socket.h>
#include <sys/time.h>
#include <sys/un.h>
#include <unistd.h>

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QThread>

#include "client.h"

// Keep in sync with SOCKET_PATH in src/config.rs.
static char const kSocketPath[] = "/tmp/kobeatctl.sock";
static int const kTimeoutSecs = 5;
static int const kMaxResponse = 1 << 20;

KobeatClient *KobeatClient::instance() {
    static KobeatClient *client = nullptr;
    if (!client) {
        QThread *thread = new QThread();
        thread->setObjectName("kobeat-ipc");
        client = new KobeatClient();
        client->moveToThread(thread);
        // The emitter is the client itself, so this queued connection always
        // hops onto the worker thread no matter who emits `send`.
        connect(client, SIGNAL(send(int, QByteArray)), client, SLOT(process(int, QByteArray)), Qt::QueuedConnection);
        thread->start();
    }
    return client;
}

QByteArray KobeatClient::unitCommand(char const *name) {
    // Unit variants of serde's externally tagged enums are bare JSON strings,
    // which QJsonDocument cannot produce at the top level.
    return QByteArray("\"") + name + "\"";
}

QByteArray KobeatClient::loadAndPlay(QString const &path) {
    QJsonObject args;
    args.insert("path", path);
    QJsonObject cmd;
    cmd.insert("LoadAndPlay", args);
    return QJsonDocument(cmd).toJson(QJsonDocument::Compact);
}

QByteArray KobeatClient::seek(qulonglong positionSecs) {
    return "{\"Seek\":{\"position_secs\":" + QByteArray::number(positionSecs) + "}}";
}

QByteArray KobeatClient::setQueue(QStringList const &paths) {
    QJsonObject args;
    args.insert("paths", QJsonArray::fromStringList(paths));
    QJsonObject cmd;
    cmd.insert("SetQueue", args);
    return QJsonDocument(cmd).toJson(QJsonDocument::Compact);
}

static QString errnoMessage(char const *what) {
    return QString("%1: %2").arg(what).arg(QString::fromLocal8Bit(strerror(errno)));
}

static QString roundTrip(QByteArray const &body, QByteArray &response) {
    int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0);
    if (fd < 0)
        return errnoMessage("socket");

    struct timeval tv = {kTimeoutSecs, 0};
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));
    setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof(tv));

    struct sockaddr_un addr;
    memset(&addr, 0, sizeof(addr));
    addr.sun_family = AF_UNIX;
    memcpy(addr.sun_path, kSocketPath, sizeof(kSocketPath));

    QString err;
    if (connect(fd, reinterpret_cast<struct sockaddr *>(&addr), sizeof(addr)) < 0) {
        err = (errno == ENOENT || errno == ECONNREFUSED)
            ? QString("kobeatd is not running")
            : errnoMessage("connect");
        close(fd);
        return err;
    }

    for (int off = 0; off < body.size();) {
        ssize_t n = send(fd, body.constData() + off, body.size() - off, MSG_NOSIGNAL);
        if (n < 0 && errno == EINTR)
            continue;
        if (n <= 0) {
            err = errnoMessage("send");
            close(fd);
            return err;
        }
        off += n;
    }
    // The daemon reads until EOF before answering.
    shutdown(fd, SHUT_WR);

    char buf[4096];
    for (;;) {
        ssize_t n = recv(fd, buf, sizeof(buf), 0);
        if (n < 0 && errno == EINTR)
            continue;
        if (n < 0) {
            err = (errno == EAGAIN || errno == EWOULDBLOCK) ? QString("kobeatd timed out") : errnoMessage("recv");
            break;
        }
        if (n == 0)
            break;
        response.append(buf, n);
        if (response.size() > kMaxResponse) {
            err = "response from kobeatd is too large";
            break;
        }
    }
    close(fd);
    if (err.isEmpty() && response.trimmed().isEmpty())
        err = "empty response from kobeatd";
    return err;
}

void KobeatClient::process(int kind, QByteArray body) {
    QByteArray response;
    QString err = roundTrip(body, response);
    if (kind == Shutdown && response.trimmed().isEmpty() && err != "kobeatd is not running") {
        err.clear();
        response = "{\"Ok\":\"kobeatd stopped\"}";
    }
    emit replied(kind, response, err);
}
