#pragma once

#include <cstddef>

class QString;
class QWidget;

// Private libnickel functions, resolved by NickelHook at load time. Every
// entry is optional: callers must check for null and degrade gracefully, so
// a firmware update that drops a symbol hides a feature instead of crashing
// Nickel. ARM C1 constructors return `this`, which we ignore.
struct KobeatNickel {
    void (*N3Dialog_ctor)(QWidget *self, QWidget *parent, bool unknown);
    void (*N3Dialog_setTitle)(QWidget *self, QString const &title);
    void (*N3Dialog_enableFullViewMode)(QWidget *self);
    void (*N3Dialog_enableBackButton)(QWidget *self, bool enable);
    void (*N3Dialog_disableCloseButton)(QWidget *self);
    void (*N3Dialog_setContent)(QWidget *self, QWidget *content);

    void (*MainNavButton_ctor)(QWidget *self, QWidget *parent);
    void (*MainNavButton_setText)(QWidget *self, QString const &text);
    void (*MainNavButton_setPixmap)(QWidget *self, QString const &path);
    void (*MainNavButton_setActivePixmap)(QWidget *self, QString const &path);
    void (*MainNavButton_setActive)(QWidget *self, bool active);

    void (*MenuTextItem_ctor)(QWidget *self, QWidget *parent, bool checkable, bool italic);
    void (*MenuTextItem_setText)(QWidget *self, QString const &text);
    void (*MenuTextItem_registerForTapGestures)(QWidget *self);

    bool hasNativeDialog() const;
    bool hasNavButton() const;
    bool hasMenuTextItem() const;
};

extern KobeatNickel kobeat_nickel;

// Zeroed storage for a libnickel object constructed in place. Nickel deletes
// these through their virtual destructors, so the memory must come from the
// same malloc-backed heap as operator new. Sizes are padded well beyond what
// firmware 4.46 needs (N3Dialog 0x44, MainNavButton 0x7c, MenuTextItem 0x60)
// to survive members being added in later firmware.
void *kobeat_native_alloc(size_t size = 1024);
