#pragma once

class QWidget;

// Makes the buttons, checkboxes and seek bars under root react to taps.
//
// Nickel's own widgets are driven by touch gestures, and on firmware 4.x
// touches aren't reliably turned into the mouse clicks that plain Qt widgets
// expect. This handles raw touch events and tap gestures directly, and drops
// any mouse events that follow a touch so a tap never fires twice. Call it
// after all the widgets under root have been created.
void kobeat_make_tappable(QWidget *root);
