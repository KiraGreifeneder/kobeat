#pragma once

#include <QByteArray>
#include <QList>
#include <QPoint>
#include <QString>
#include <QStringList>
#include <QWidget>

class QCheckBox;
class QLabel;
class QPushButton;
class QTimer;

// Horizontal progress bar; tapping it requests a seek to that fraction.
class KobeatSeekBar : public QWidget {
    Q_OBJECT

public:
    explicit KobeatSeekBar(QWidget *parent = nullptr);
    void setFraction(double fraction);
    // Seeks to the fraction under pos (in widget coordinates).
    void tapAt(QPoint const &pos);

signals:
    void seekRequested(double fraction);

protected:
    void paintEvent(QPaintEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;

private:
    double m_fraction;
};

// The player: transport controls, now playing, and a paged browser for the
// music directory. Built from plain Qt widgets so it works inside either a
// native N3Dialog or a fallback top-level window. Paging (instead of
// scrolling) keeps it usable on e-ink.
class KobeatPlayerView : public QWidget {
    Q_OBJECT

public:
    explicit KobeatPlayerView(QWidget *parent = nullptr);

protected:
    void showEvent(QShowEvent *event) override;
    void hideEvent(QHideEvent *event) override;
    bool eventFilter(QObject *obj, QEvent *event) override;

private slots:
    void poll();
    void onReplied(int kind, QByteArray response, QString error);
    void onPlayPause();
    void onPrevious();
    void onNext();
    void onSeek(double fraction);
    void onRowTapped();
    void onUp();
    void onPrevPage();
    void onNextPage();
    void onPlayFolder();
    void onDaemonButton();
    void onDaemonSettled();

private:
    void command(QByteArray const &body);
    void applyState(QByteArray const &response);
    void setStatus(QString const &text);
    void setDaemonState(int state);
    void updateDaemonButton();
    void openDir(QString const &dir);
    void renderPage();
    void layoutRows();
    QStringList collectFiles(QString const &dir, bool recursive, int depth) const;

    QTimer *m_poll;
    int m_stateInFlight;
    bool m_connectionError;

    enum DaemonState { DaemonUnknown, DaemonRunning, DaemonStopped };
    int m_daemonState;
    // Set briefly after start/stop so a double tap can't launch a second
    // kobeatd (which would steal the first one's socket).
    bool m_daemonBusy;
    QPushButton *m_daemonButton;

    QLabel *m_status;
    QLabel *m_title;
    KobeatSeekBar *m_seek;
    QLabel *m_time;
    QPushButton *m_playPause;
    QString m_playback;
    double m_duration;

    QLabel *m_dirLabel;
    QPushButton *m_up;
    // Holds the row buttons; as many rows are shown as fit its height, so the
    // view never grows past the space Nickel gives it.
    QWidget *m_list;
    int m_rowHeight;
    int m_rowsPerPage;
    QList<QPushButton *> m_rows;
    QPushButton *m_prevPage;
    QLabel *m_pageLabel;
    QPushButton *m_nextPage;
    QCheckBox *m_recursive;

    QString m_dir;
    QStringList m_entries; // directories end with '/'
    int m_page;
};
