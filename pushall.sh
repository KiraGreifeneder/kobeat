#!/bin/zsh

source $HOME/.zshrc

kobopush ./target/armv7-unknown-linux-musleabihf/release/kobeatui 10.20.40.10
kobopush ./target/armv7-unknown-linux-musleabihf/release/kobeatd 10.20.40.10
kobopush ./target/armv7-unknown-linux-musleabihf/release/kobeatctl 10.20.40.10
