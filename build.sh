#!/bin/sh
# Creates a distributable tarball for a local platform or for
# a given target triple. To build for the local platform, do not
# pass any arguments:
#
#     ./build.sh
#
# The result is a tarball with a name like `fernspielapparat-0.1.0-linux-x86_64.tar.gz`,
# which is assembled from uname info.
#
# To cross-compile with an ARM toolchain to get a tarball like
# `release/fernspielapparat-<version>-arm-unknown-linux-gnueabihf.tar.gz`,
# invoke the script with the desired target triple:
#
#     ./build.sh arm-unknown-linux-gnueabihf

METADATA=$(head -n 4 Cargo.toml | sed -n 's/^.*"\([^"]*\)".*$/\1/p')
CRATE=$(echo "$METADATA" | head -n 1)
VERSION=$(echo "$METADATA" | sed -n '2p')

HOST_SYSTEM_NAME=$(uname | sed -e 's/\(.*\)/\L\1/')    # lower-case name like "linux"
HOST_SYSTEM_ARCH=$(uname -m | sed -e 's/\(.*\)/\L\1/') # lower-case, e.g. "x86_64"

CARGO_ARGS="--release"
if [ -z "$1" ]
then
    echo "Building for host system architecture ..."
    BINARY="target/release/$CRATE"
    RELEASE_DIR_NAME="$CRATE-$VERSION-$HOST_SYSTEM_NAME-$HOST_SYSTEM_ARCH"
else
    CROSS_TRIPLE="$1"
    echo "Cross-compiling for achitecture $CROSS_TRIPLE ..."
    CARGO_ARGS="$CARGO_ARGS --target=$CROSS_TRIPLE"
    BINARY="target/$CROSS_TRIPLE/release/$CRATE"
    RELEASE_DIR_NAME="$CRATE-$VERSION-$CROSS_TRIPLE"
fi

RELEASE_DIR="release/$RELEASE_DIR_NAME"
RELEASE_TAR="$RELEASE_DIR_NAME.tar.gz" # fernspielapparat-0.1.0-linux-x86_64.tar.gz

echo "Building $CRATE into $RELEASE_DIR"

function clean {
    rm -rf $RELEASE_DIR && \
    mkdir -p $RELEASE_DIR
}

function copy_assets {
    cp AUTHORS.md $RELEASE_DIR && \
    cp COMPILE.md $RELEASE_DIR && \
    cp LICENSE $RELEASE_DIR && \
    cp README.md $RELEASE_DIR && \
    cp -r doc $RELEASE_DIR && \
    cp -r examples $RELEASE_DIR
}

function generate_source_link {
    echo "The source code is publicly hosted at GitHub:
https://github.com/krachzack/fernspielapparat" > $RELEASE_DIR/SOURCE
}

cargo build $CARGO_ARGS && \
echo "Clearing output directory ..." && \
clean && \
echo "Copying binary ..." && \
cp $BINARY $RELEASE_DIR || cp $BINARY.exe $RELEASE_DIR && \
echo "Copying static assets ..." && \
copy_assets && \
generate_source_link && \
echo "Writing compressed tarball $RELEASE_TAR ..." && \
cd release && \
tar -zcf $RELEASE_TAR $RELEASE_DIR_NAME && \
cd ..
