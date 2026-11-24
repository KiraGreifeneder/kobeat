#include <cerrno>
#include <cstring>

#include <fcntl.h>
#include <sys/wait.h>
#include <unistd.h>

#include <QFileInfo>

#include "daemon.h"

static char const *const kDaemonPaths[] = {
    "/mnt/onboard/.adds/kobeat/kobeatd",
    "/mnt/onboard/kobeatd",
};
static char const kDaemonLog[] = "/tmp/kobeatd.log";

QString kobeat_daemon_path() {
    for (size_t i = 0; i < sizeof(kDaemonPaths) / sizeof(*kDaemonPaths); i++)
        if (QFileInfo(QString::fromLatin1(kDaemonPaths[i])).isFile())
            return QString::fromLatin1(kDaemonPaths[i]);
    return QString();
}

QString kobeat_start_daemon() {
    QString path = kobeat_daemon_path();
    if (path.isEmpty())
        return QString("kobeatd not found (put it at %1)").arg(QString::fromLatin1(kDaemonPaths[0]));

    // Everything the children need is prepared up front: between fork and
    // exec in a multithreaded process only async-signal-safe calls are allowed.
    QByteArray file = path.toLocal8Bit();
    char *const argv[] = {file.data(), nullptr};
    long maxFd = sysconf(_SC_OPEN_MAX);
    if (maxFd < 0 || maxFd > 65536)
        maxFd = 1024;

    // The grandchild reports an exec failure through this pipe; on success
    // O_CLOEXEC closes it and the parent reads EOF.
    int report[2];
    if (pipe2(report, O_CLOEXEC) < 0)
        return QString("pipe: %1").arg(QString::fromLocal8Bit(strerror(errno)));

    pid_t child = fork();
    if (child < 0) {
        int err = errno;
        close(report[0]);
        close(report[1]);
        return QString("fork: %1").arg(QString::fromLocal8Bit(strerror(err)));
    }
    if (child == 0) {
        setsid();
        if (fork() != 0)
            _exit(0); // reparent the daemon to init
        int devnull = open("/dev/null", O_RDONLY);
        int log = open(kDaemonLog, O_WRONLY | O_CREAT | O_APPEND, 0644);
        if (devnull >= 0)
            dup2(devnull, 0);
        if (log >= 0) {
            dup2(log, 1);
            dup2(log, 2);
        }
        for (long fd = 3; fd < maxFd; fd++)
            if (fd != report[1])
                close(fd);
        execv(argv[0], argv);
        int err = errno;
        ssize_t ignored = write(report[1], &err, sizeof(err));
        (void)ignored;
        _exit(127);
    }

    close(report[1]);
    // Nickel may reap children itself; ECHILD here is harmless.
    while (waitpid(child, nullptr, 0) < 0 && errno == EINTR) {
    }
    int err = 0;
    ssize_t n;
    while ((n = read(report[0], &err, sizeof(err))) < 0 && errno == EINTR) {
    }
    close(report[0]);
    if (n == sizeof(err))
        return QString("could not run %1: %2").arg(path).arg(QString::fromLocal8Bit(strerror(err)));
    return QString();
}
