#!/bin/zsh

source $HOME/.zshrc

# kobopush ./target/armv7-unknown-linux-musleabihf/release/kobeat-native-ui 10.20.40.10
kobopush ./target/armv7-unknown-linux-musleabihf/release/kobeatd 10.20.40.10
kobopush ./target/armv7-unknown-linux-musleabihf/release/kobeatctl 10.20.40.10
# Nickel plugin; installed by Nickel on the next reboot. The Kobo's ssh ignores
# remote commands, so send a script on stdin like kobo_push does.
echo "Pushing KoboRoot.tgz to Kobo..."
(
    echo "base64 -d > /mnt/onboard/.kobo/KoboRoot.tgz << '__KOBO_EOF__'"
    base64 ./src/kobeatnickel/KoboRoot.tgz
    echo "__KOBO_EOF__"
    echo "ls -l /mnt/onboard/.kobo/KoboRoot.tgz"
) | ssh root@10.20.40.10
