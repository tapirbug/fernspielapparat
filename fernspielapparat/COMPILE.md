# Cross compiling for Raspberry Pi 2

## Arch Linux
From the AUR. E.g. with aurman (you can of course do this manually):

    aurman -Syu arm-linux-gnueabihf-binutils \
        arm-linux-gnueabihf-gcc-stage1 && \
    aurman -Syu arm-linux-gnueabihf-linux-api-headers \
        arm-linux-gnueabihf-glibc-headers && \
    aurman -Syu arm-linux-gnueabihf-gcc-stage2 && \
    aurman -Syu arm-linux-gnueabihf-glibc \
        arm-linux-gnueabihf-gcc