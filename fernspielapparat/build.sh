#!/bin/sh
# Builds the runtime for Raspberry Pi 2

# RPi2-compatible ARM
CROSS_TARGET="arm-unknown-linux-gnueabihf"
LIBS_DIR="/usr/arm-linux-gnueabihf/lib"
#RUSTFLAGS="-L $LIBS_DIR"

# cd to fernspielapparat
cd $(cd -P -- "$(dirname -- "$0")" && pwd -P)

# And build for CROSS_TARGET
cargo build --verbose --release --target=$CROSS_TARGET
