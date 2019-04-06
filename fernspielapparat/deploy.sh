#!/bin/sh
# Builds and deploys the runtime to Raspberry Pi 2

# RPi2-compatible ARM
RPI_USER="pi"
RPI_HOSTNAME="rudi"
RPI_DIR="~"
SCP_DEST="$RPI_USER@$RPI_HOSTNAME:$RPI_DIR"
CROSS_TRIPLE="arm-unknown-linux-gnueabihf"

# cd to fernspielapparat
cd $(cd -P -- "$(dirname -- "$0")" && pwd -P) && \

# And build for CROSS_TRIPLE
cargo build --verbose --release --target=$CROSS_TRIPLE && \

# Copy to Raspberry Pi via scp
echo "Build successful, deploying binary to $RPI_HOSTNAME..." && \
scp target/$CROSS_TRIPLE/release/fernspielapparat $SCP_DEST
