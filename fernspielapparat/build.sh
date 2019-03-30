#!/bin/sh
# Builds the runtime for Raspberry Pi 2

# cd to fernspielapparat
cd $(cd -P -- "$(dirname -- "$0")" && pwd -P)

source scripts/setup_toolchain_rp2.sh

# cargo build --target=$CROSS_TARGET
