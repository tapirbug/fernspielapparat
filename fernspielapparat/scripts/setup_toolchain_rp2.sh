#!/bin/bash
# Ensures that cargo can compile for the Raspberry Pi 2.
# It sets $CROSS_TARGET to the target to pass to cargo, e.g.
# 
#     cargo --target=$CROSS_TARGET build
#
# If RP2 is not the native architecture, tries to install
# a cross compile toolchain and returns an unsuccessful exit
# status if failed.

if grep -q BCM2708 /proc/cpuinfo
then
    echo "This seems to be a Raspberry Pi 2 or something like it, skipping cross compile toolchain install."
    exit 0
fi

export CROSS_TARGET="arm-unknown-linux-gnueabihf"

function panic {
    echo -e "fatal: $1"
    exit 1
}

function confirm {
    echo $1
    select yn in "Proceed" "Cancel"; do
        case $yn in
            Proceed ) return 0; break;;
            Cancel ) panic "Denied, exiting." ;;
        esac
    done
}

rustup target install $CROSS_TARGET \
    || panic "Failed to add rustup target"

# On arch linux, use AUR package
if pacman --version && makepkg --version
then
    CROSS_DIRS_PARENT="archlinux-aur-$CROSS_TARGET"
    DIR_BINUTILS="arm-linux-gnueabihf-binutils"
    DIR_GLIBC="arm-linux-gnueabihf-glibc"
    DIR_GCC="arm-linux-gnueabihf-gcc"
    DIR_LINUX_API_HEADERS="arm-linux-gnueabihf-linux-api-headers"

    if true || [ ! -d "$CROSS_DIRS_PARENT/$DIR_BINUTILS" ] || [ ! -d "$CROSS_DIRS_PARENT/$DIR_GLIBC" ] || [ ! -d "$CROSS_DIRS_PARENT/$DIR_GCC" ] || [ ! -d "$CROSS_DIRS_PARENT/$DIR_LINUX_API_HEADERS" ]
    then
        confirm "Building toolchain into ./$CROSS_DIRS_PARENT and installing. This may require sudo. Proceed?"
        echo "Confirmed, installing cross compile toolchain..."

        mkdir -p $CROSS_DIRS_PARENT \
            && cd $CROSS_DIRS_PARENT \
            || panic "Could not create directory for cross compile dependencies"

        # Clone repos
        [ -d $DIR_LINUX_API_HEADERS ] \
            || git clone "https://aur.archlinux.org/$DIR_LINUX_API_HEADERS.git" \
            || panic "Could not obtain cross compile toolchain from AUR"

        [ -d $DIR_GLIBC ] \
            || git clone "https://aur.archlinux.org/$DIR_GLIBC.git" \
            || panic "Could not obtain cross compile toolchain from AUR"

        [ -d $DIR_BINUTILS ] \
            || git clone "https://aur.archlinux.org/$DIR_BINUTILS.git" \
            || panic "Could not obtain cross compile toolchain from AUR"


        [ -d $DIR_GCC ] \
            || git clone "https://aur.archlinux.org/$DIR_GCC.git" \
            || panic "Could not obtain cross compile toolchain from AUR"
        

        # Make packages in order and install
        cd $DIR_LINUX_API_HEADERS \
            && makepkg -si \
            && cd .. \
            || panic "Could not make and install cross compile toolchain package dependency linux api headers"

        cd $DIR_GLIBC \
            && makepkg -si \
            && cd .. \
            || panic "Could not make and install cross compile toolchain package dependency glibc"

        cd $DIR_GCC \
            && makepkg -si \
            && cd .. \
            || panic "Could not make and install cross compile toolchain package"
        
        cd $DIR_BINUTILS \
            && makepkg -si \
            && cd .. \
            || panic "Could not make and install cross compile toolchain package dependency binutils"
        
        cd ..
    fi
# On debian-like try apt-get
elif apt-get --version
then
    confirm "Installing cross compile toolchain via apt-get. Proceed?"
    sudo apt-get install gcc-arm-linux-gnueabihf \
        || panic "Failed to install cross compile toolchain"
fi
