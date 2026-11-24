#include <cstdlib>

#include "nickel.h"

KobeatNickel kobeat_nickel = {};

bool KobeatNickel::hasNativeDialog() const {
    return N3Dialog_ctor && N3Dialog_setTitle && N3Dialog_enableFullViewMode && N3Dialog_enableBackButton
        && N3Dialog_disableCloseButton && N3Dialog_setContent;
}

bool KobeatNickel::hasNavButton() const {
    return MainNavButton_ctor && MainNavButton_setText;
}

bool KobeatNickel::hasMenuTextItem() const {
    return MenuTextItem_ctor && MenuTextItem_setText && MenuTextItem_registerForTapGestures;
}

void *kobeat_native_alloc(size_t size) {
    return calloc(1, size);
}
