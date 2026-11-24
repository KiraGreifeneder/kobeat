# kobeat
Listen to music on your kobo while you're reading, or listen to audiobooks!

## Included Binaries

### kobeatd

The Kobeat daemon that listens for commands and plays audio in the background.
Music goes in `/mnt/onboard/music` (the `music` folder on the Kobo's USB
storage); kobeatd and the Nickel plugin create it if it doesn't exist yet.

### kobeatctl 

An optional simple command line tool for sending commands to kobeatd.

### libkobeat.so (Nickel plugin, `src/kobeatnickel`)

The native UI that runs inside Kobo's own reading app (Nickel). It
adds a **Kobeat** tab to the home screen's bottom bar and a **Kobeat** item
to the reader menu, and it can also be opened from NickelMenu, KFMon, or any
shell command (see [Opening the native player](#opening-the-native-player)).
All of these open a full-screen player that covers the bottom bar while it's
open, with play/pause, previous/next, tap-to-seek, a music folder browser, and
a **Start/Stop kobeatd** button. It talks to kobeatd directly over its socket.

It doesn't need NickelMenu or Nickelite, and it works alongside them because it
doesn't hook any Nickel functions. It's built for firmware 4.x (Qt 5.2.1) and
has been checked against 4.46. If a later firmware is missing a Nickel feature
the plugin uses, the plugin hides the matching entry instead of crashing Nickel.


## Compiling for Kobo Libra Color (and possibly others)

You will need to install the `cross` tool to cross-compile for the Kobo's ARM architecture.

```bash
cargo install cross --locked
```

To build:

```bash
cross build --bin kobeatd --bin kobeatctl --bin kobeat-native-ui --target armv7-unknown-linux-musleabihf --release
```

The first build might take a while, since it pulls a podman/docker container and compiles the Rust toolchain for the target architecture. Subsequent builds will be faster as the container is reused.

The final binaries (kobeatd, kobeatctl, kobeat-native-ui) will be located in `target/armv7-unknown-linux-musleabihf/release/` and can be copied to the Kobo using `scp` or similar tools.

kobeatd is the daemon that runs on the Kobo, while kobeatctl is a command-line tool for controlling the daemon from your computer. You can use kobeatctl to play, pause, skip tracks, and manage your music library on the Kobo.

### Building the Nickel plugin

The plugin is built with [NickelHook](https://github.com/pgaskin/NickelHook)
(a git submodule) and the NickelTC toolchain container:

```bash
git submodule update --init src/kobeatnickel/NickelHook
cd src/kobeatnickel
podman run --rm -v "$PWD":/src:Z -w /src ghcr.io/pgaskin/nickeltc:1.0 sh -c 'make && make koboroot'
```

(`docker` works the same way.) This produces `src/kobeatnickel/KoboRoot.tgz`.
To install it, copy that file into the `.kobo/` folder on the Kobo's USB
storage and eject the device, or copy it over SSH
(base64-encoded through `ssh`, which
is what `pushall.sh` does) and reboot. The Kobo installs the plugin to
`/usr/local/Kobo/imageformats/libkobeat.so` and restarts.

To uninstall, create an empty file at `.adds/kobeat/uninstall` on the Kobo's
storage and reboot. NickelHook's failsafe also removes the plugin on its own
if Nickel crashes while loading it. Plugin logs go to syslog with the
`Kobeat` prefix (`logread | grep Kobeat`).

### Opening the native player

Besides the built-in entries, the plugin reads commands from the FIFO
`/tmp/kobeat-ui.fifo`. Write `open`, `close`, or `toggle`, one per line:

```bash
echo open > /tmp/kobeat-ui.fifo
```

NickelMenu (`.adds/nm/config`):

```
menu_item :main   :Kobeat :cmd_spawn :quiet:echo open > /tmp/kobeat-ui.fifo
menu_item :reader :Kobeat :cmd_spawn :quiet:echo open > /tmp/kobeat-ui.fifo
```

KFMon: point a watch's `action` at a script containing the same `echo`
line.

The FIFO is created when Nickel starts with the plugin loaded. If the plugin
isn't installed, `echo` just creates a regular file there, which the plugin
replaces on its next start.

To hide the built-in entries, for example when you launch the player from
NickelMenu instead, create these empty files and restart Nickel:

- `.adds/kobeat/no_nav_tab` hides the bottom bar tab.
- `.adds/kobeat/no_reader_menu` hides the reader menu item.

### Starting and stopping kobeatd from the player

The plugin doesn't start kobeatd on its own. The button next to the status
line at the top of the player shows **Start kobeatd** when the daemon isn't
running and **Stop kobeatd** when it is.

- **Start** launches the first of these that exists, detached from Nickel:
  `.adds/kobeat/kobeatd` or `kobeatd` in the root of the Kobo's storage
  (`/mnt/onboard/.adds/kobeat/kobeatd`, `/mnt/onboard/kobeatd`; `pushall.sh`
  uses the latter). Its output is appended to `/tmp/kobeatd.log`.
- **Stop** sends kobeatd the `Shutdown` command, which stops playback and
  exits the daemon.

The button is disabled for a few seconds after each tap, so you can't start
two daemons by accident.



## Usage

Copy kobeatd (and optionally kobeatctl) to the Kobo, install the Nickel
plugin, and put your music in the `music` folder.

Start kobeatd from the player's **Start kobeatd** button (see
[Starting and stopping kobeatd from the player](#starting-and-stopping-kobeatd-from-the-player)),
or run it automatically on startup, e.g. from a KFMon or NickelMenu script, or
a udev rule.


## Local Testing

For now there's no dedicated virtual environment to test the binaries locally.
However, you can run the binaries directly on your system using `cargo run` if you're on linux and have the necessary dependencies installed.

e.g. `cargo run --bin kobeatd`, `cargo run --bin kobeatctl`

## Modules

### /bin

The `bin` directory contains the main entry points for the daemon and CLI.

### /daemon

The `daemon` directory contains the main daemon code.
This is where the daemon's main loop and event handling logic is implemented.
It opens the socket and listens for incoming requests from the CLI and the Nickel plugin, and owns the player (`daemon/player`, including the Symphonia decoder) that plays the actual audio file.

### /entity

The `entity` directory contains the shared data models used by the modules.

### /repository

Contains the repository layer, which is responsible for data persistence and retrieval.
This includes things like playlist management.

### /kobeatnickel

The Nickel plugin (C++/Qt), built separately; see
[Building the Nickel plugin](#building-the-nickel-plugin).
