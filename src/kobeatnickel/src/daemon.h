#pragma once

#include <QString>

// Where the plugin looks for the kobeatd binary, in order.
QString kobeat_daemon_path();

// Launches kobeatd fully detached from Nickel (own session, no inherited file
// descriptors, output appended to /tmp/kobeatd.log). Returns an error message,
// or an empty string once the binary has been exec'd.
QString kobeat_start_daemon();
