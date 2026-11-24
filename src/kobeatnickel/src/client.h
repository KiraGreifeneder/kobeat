#pragma once

#include <QByteArray>
#include <QObject>
#include <QString>
#include <QStringList>

// Talks to kobeatd over its Unix socket. One instance lives on a dedicated
// worker thread for the lifetime of Nickel, so blocking socket I/O (the daemon
// can take ~200ms to accept) never stalls the GUI. Requests are processed in
// order, which callers rely on (e.g. SetQueue followed by PlayNext).
class KobeatClient : public QObject {
    Q_OBJECT

public:
    // Shutdown is a Command whose missing reply counts as success, since
    // kobeatd exits without answering it.
    enum Kind { State, Command, Shutdown };

    // The shared instance, created (with its thread) on first use. Must be
    // called from the GUI thread.
    static KobeatClient *instance();

    static QByteArray unitCommand(char const *name);
    static QByteArray loadAndPlay(QString const &path);
    static QByteArray seek(qulonglong positionSecs);
    static QByteArray setQueue(QStringList const &paths);

signals:
    // Queue a raw JSON command. Thread-safe; replies arrive via `replied`.
    void send(int kind, QByteArray body);

    // `error` is empty on success, in which case `response` is the raw JSON
    // reply from the daemon.
    void replied(int kind, QByteArray response, QString error);

private slots:
    void process(int kind, QByteArray body);

private:
    KobeatClient() {}
};
