#!/bin/sh
# Creates a distributable tarball for the local platform.
# If a cross-compile toolchain is available, also builds for ARM.

METADATA=$(head -n 4 Cargo.toml | sed -n 's/^.*"\([^"]*\)".*$/\1/p')
CRATE=$(echo "$METADATA" | head -n 1)
VERSION=$(echo "$METADATA" | sed -n '2p')

HOST_SYSTEM_NAME=$(uname | sed -e 's/\(.*\)/\L\1/')    # lower-case name like "linux"
HOST_SYSTEM_ARCH=$(uname -m | sed -e 's/\(.*\)/\L\1/') # lower-case, e.g. "x86_64"
RELEASE_DIR_NAME="$CRATE-$VERSION"
RELEASE_DIR="release/$RELEASE_DIR_NAME"
RELEASE_TAR="$RELEASE_DIR_NAME-$HOST_SYSTEM_NAME-$HOST_SYSTEM_ARCH.tar.gz" # fernspielapparat-0.1.0-linux-x86_64.tar.gz

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

echo "Building for host system architecture ..." && \
cargo build --release && \
echo "Clearing output directory ..." && \
clean && \
echo "Copying binary ..." && \
cp target/release/$CRATE $RELEASE_DIR || cp target/release/$CRATE.exe $RELEASE_DIR && \
echo "Copying static assets ..." && \
copy_assets && \
generate_source_link && \
echo "Writing compressed tarball $RELEASE_TAR ..." && \
cd release && \
tar -zcf $RELEASE_TAR $RELEASE_DIR_NAME && \
cd ..
