# #!/usr/bin/env bash
dpkg --add-architecture $CROSS_DEB_ARCH
apt-get update && apt-get --assume-yes install libudev-dev:$CROSS_DEB_ARCH