# kobeat

Listen to music and audiobooks on your Kobo while you read.

kobeat plays audio in the background over the Kobo's Bluetooth output, and adds
a native player to Kobo's own reading app so you can control it without leaving
your book. It's tested on the Kobo Libra Colour with firmware 4.46.

- **kobeatd**: a background daemon that decodes and plays audio.
- **kobeatctl**: a small command line client for controlling the daemon.
- **libkobeat.so**: a Nickel plugin (`src/kobeatnickel`) that adds the player UI.

## Install and use

Download the latest zip from the
[releases page](https://github.com/KiraGreifeneder/kobeat/releases), unzip it
onto the Kobo's USB storage, and eject. See the [user guide](GUIDE.md) for
adding music, using the player, and uninstalling.

## Building

Cross-compile the daemon and CLI with [`cross`](https://github.com/cross-rs/cross):

```bash
cargo install cross --locked
cross build --release --target armv7-unknown-linux-musleabihf --bin kobeatd --bin kobeatctl
```

Build the Nickel plugin with the [NickelTC](https://github.com/pgaskin/nickeltc)
container (produces `src/kobeatnickel/KoboRoot.tgz`):

```bash
git submodule update --init src/kobeatnickel/NickelHook
cd src/kobeatnickel
podman run --rm -v "$PWD":/src:Z -w /src ghcr.io/pgaskin/nickeltc:1.0 sh -c 'make && make koboroot'
```

`docker` works the same way. You can also run the daemon and CLI on a Linux
machine with `cargo run --bin kobeatd` / `cargo run --bin kobeatctl`.

## Future features

- [ ] Other UI implementations to support kobeat in KOReader, for example.
- [ ] Testing/support on other Kobo devices
- [ ] Testing/support for newer firmware
- [ ] Improve player responsiveness

## License

[MIT](LICENSE)
