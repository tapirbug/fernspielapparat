#!/bin/sh
# Builds and deploys the runtime to Raspberry Pi 2

# RPi2-compatible ARM
RPI_USER="pi"
RPI_HOSTNAME="rudi"
RPI_DIR="~"
SSH_HOST="$RPI_USER@$RPI_HOSTNAME"
SCP_DEST="$SSH_HOST:$RPI_DIR"
CROSS_TRIPLE="arm-unknown-linux-gnueabihf"

# cd to fernspielapparat
cd $(cd -P -- "$(dirname -- "$0")" && pwd -P) && \

# And build for CROSS_TRIPLE
cargo build --verbose --release --target=$CROSS_TRIPLE || exit 1

# Copy to Raspberry Pi via scp
echo "Build successful, deploying binary to $RPI_HOSTNAME..."

# Stop service for upgrade first
ssh $SSH_HOST 'sudo systemctl stop fernspielapparat'

# Update unit file and executable, restart service
scp scripts/fernspielapparat.service $SCP_DEST && \
scp target/$CROSS_TRIPLE/release/fernspielapparat $SCP_DEST && \
ssh $SSH_HOST 'sudo mv -f fernspielapparat.service /etc/systemd/system && sudo systemctl enable fernspielapparat && sudo systemctl start fernspielapparat'

