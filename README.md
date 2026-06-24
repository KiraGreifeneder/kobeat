# kobeat
Listen to music on your kobo while you're reading, or listen to audiobooks!

## Compiling for Kobo Libra Color (and possibly others)

```bash
cargo install cross --locked
```

To build:

```bash
cross build --bin kobeatd --bin kobeatctl --bin kobeatui --target armv7-unknown-linux-musleabihf --release
```

The first build might take a while, since it pulls a podman/docker container and compiles the Rust toolchain for the target architecture. Subsequent builds will be faster as the container is reused.

The final binaries will be located in `target/armv7-unknown-linux-gnueabihf/release/` and can be copied to the Kobo using `scp` or similar tools.

kobeatd is the daemon that runs on the Kobo, while kobeatctl is a command-line tool for controlling the daemon from your computer. You can use kobeatctl to play, pause, skip tracks, and manage your music library on the Kobo.
