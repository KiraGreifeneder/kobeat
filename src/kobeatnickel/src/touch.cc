#include <QAbstractButton>
#include <QElapsedTimer>
#include <QEvent>
#include <QGestureEvent>
#include <QPointer>
#include <QTapGesture>
#include <QTouchEvent>
#include <QWidget>

#include "player_view.h"
#include "touch.h"

namespace {

// A finger that moves further than this is scrolling/swiping, not tapping.
int const kSlop = 40;
// Mouse events this soon after a touch are the synthesized copy of it.
int const kMouseSuppressMs = 800;
// The touch and gesture paths both report the same tap.
int const kDuplicateMs = 400;

class TapFilter : public QObject {
public:
    explicit TapFilter(QObject *parent) : QObject(parent), m_cancelled(false) {}

    bool eventFilter(QObject *obj, QEvent *event) override {
        QWidget *w = static_cast<QWidget *>(obj);
        switch (event->type()) {
        case QEvent::TouchBegin: {
            QList<QTouchEvent::TouchPoint> points = static_cast<QTouchEvent *>(event)->touchPoints();
            m_lastTouch.start();
            if (points.isEmpty())
                return true;
            m_target = w;
            m_start = points.first().screenPos().toPoint();
            m_cancelled = false;
            setDown(w, true);
            event->accept();
            return true;
        }
        case QEvent::TouchUpdate: {
            QList<QTouchEvent::TouchPoint> points = static_cast<QTouchEvent *>(event)->touchPoints();
            m_lastTouch.start();
            if (w == m_target && !m_cancelled && !points.isEmpty() &&
                (points.first().screenPos().toPoint() - m_start).manhattanLength() > kSlop) {
                m_cancelled = true;
                setDown(w, false);
            }
            return true;
        }
        case QEvent::TouchEnd: {
            QList<QTouchEvent::TouchPoint> points = static_cast<QTouchEvent *>(event)->touchPoints();
            m_lastTouch.start();
            if (w == m_target) {
                setDown(w, false);
                if (!m_cancelled && !points.isEmpty()) {
                    QPoint pos = points.first().pos().toPoint();
                    if (w->rect().contains(pos))
                        fire(w, pos);
                }
            }
            m_target = nullptr;
            return true;
        }
        case QEvent::TouchCancel:
            setDown(w, false);
            m_target = nullptr;
            return true;
        case QEvent::Gesture: {
            QGestureEvent *ge = static_cast<QGestureEvent *>(event);
            if (QGesture *g = ge->gesture(Qt::TapGesture)) {
                ge->accept(g);
                if (g->state() == Qt::GestureFinished) {
                    QPoint pos = g->hasHotSpot() ? w->mapFromGlobal(g->hotSpot().toPoint()) : w->rect().center();
                    if (!w->rect().contains(pos))
                        pos = w->rect().center();
                    fire(w, pos);
                }
                return true;
            }
            return false;
        }
        case QEvent::MouseButtonPress:
        case QEvent::MouseButtonRelease:
        case QEvent::MouseButtonDblClick:
        case QEvent::MouseMove:
            return m_lastTouch.isValid() && m_lastTouch.elapsed() < kMouseSuppressMs;
        default:
            return false;
        }
    }

private:
    static void setDown(QWidget *w, bool down) {
        if (QAbstractButton *b = qobject_cast<QAbstractButton *>(w))
            b->setDown(down && b->isEnabled());
    }

    void fire(QWidget *w, QPoint const &pos) {
        if (w == m_lastFired && m_lastFire.isValid() && m_lastFire.elapsed() < kDuplicateMs)
            return;
        m_lastFired = w;
        m_lastFire.start();
        m_lastTouch.start();
        if (!w->isEnabled() || !w->isVisible())
            return;
        if (QAbstractButton *b = qobject_cast<QAbstractButton *>(w))
            b->click();
        else if (KobeatSeekBar *s = qobject_cast<KobeatSeekBar *>(w))
            s->tapAt(pos);
    }

    QPointer<QWidget> m_target;
    QPoint m_start;
    bool m_cancelled;
    QElapsedTimer m_lastTouch;
    QPointer<QWidget> m_lastFired;
    QElapsedTimer m_lastFire;
};

} // namespace

void kobeat_make_tappable(QWidget *root) {
    TapFilter *filter = new TapFilter(root);
    QList<QWidget *> widgets = root->findChildren<QWidget *>();
    widgets.prepend(root);
    foreach (QWidget *w, widgets) {
        if (!qobject_cast<QAbstractButton *>(w) && !qobject_cast<KobeatSeekBar *>(w))
            continue;
        w->setAttribute(Qt::WA_AcceptTouchEvents, true);
        w->grabGesture(Qt::TapGesture);
        w->installEventFilter(filter);
    }
}
