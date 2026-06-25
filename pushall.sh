#!/bin/zsh

source $HOME/.zshrc

kobo_push ./target/armv7-unknown-linux-musleabihf/release/kobeatui
kobo_push ./target/armv7-unknown-linux-musleabihf/release/kobeatd
kobo_push ./target/armv7-unknown-linux-musleabihf/release/kobeatctl
