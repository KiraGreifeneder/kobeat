# kobeat
Listen to music on your kobo while you're reading, or listen to audiobooks!

## Included Binaries

### kobeatd

The Kobeat daemon that listens for commands and plays audio in the background

### kobeatui

A web server for providing a UI for Kobeat. It uses the same underlying control library as kobeatctl.

### kobeatctl 

An optional simple command line tool for sending commands to kobeatd.


## Compiling for Kobo Libra Color (and possibly others)

```bash
cargo install cross --locked
```

To build:

```bash
cross build --bin kobeatd --bin kobeatctl --bin kobeatui --target armv7-unknown-linux-musleabihf --release
```

The first build might take a while, since it pulls a podman/docker container and compiles the Rust toolchain for the target architecture. Subsequent builds will be faster as the container is reused.

The final binaries (kobeatd, kobeatctl, kobeatui) will be located in `target/armv7-unknown-linux-gnueabihf/release/` and can be copied to the Kobo using `scp` or similar tools.

kobeatd is the daemon that runs on the Kobo, while kobeatctl is a command-line tool for controlling the daemon from your computer. You can use kobeatctl to play, pause, skip tracks, and manage your music library on the Kobo.

## Local Testing

For now there's no dedicated virtual environment to test the binaries locally.
However, you can run the binaries directly on your system using `cargo run` if you're on linux and have the necessary dependencies installed.

e.g. `cargo run --bin kobeatd`, `cargo run --bin kobeatctl`, `cargo run --bin kobeatui`

## Modules

### /bin

The `bin` directory contains the main entry points for the daemon, CLI, and UI.

### /daemon

The `daemon` directory contains the main daemon code.
This is where the daemon's main loop and event handling logic is implemented.
It opens the socket and listens for incoming requests from the CLI and UI, and owns the player that plays the actual audio file.

### /entity

The `entity` directory contains the shared data models used by the modules.

### /repository

Contains the repository layer, which is responsible for data persistence and retrieval.
This includes things like playlist management.

### /ui

The UI directory owns the server and client code for kobeatui.

## Usage

Place both files on your Kobo's file system.

```
# Browser menu item for exiting the browser
menu_item :browser :Exit Browser :nickel_misc :home

# Main menu item for launching the browser in a modal with the kobeat ui
menu_item :main :Kobeat :nickel_browser :modal:http://127.0.0.1:3000

```

Set `kobeatd` to run automatically on startup. E.g. systemctl start
