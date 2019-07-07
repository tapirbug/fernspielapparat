# Compiling the fernspiealapparat Runtime
This guide is supposed to help you compile the fernspielapparat
runtime for your host system or by using cross-compilation to
compile it for deployment on a Raspberry Pi 2 device.

## Compile for Your Own Computer
The following steps enable you to compile the runtime locally.

### Build System
The rust build system cargo is required. Download it with
[rustup](https://rustup.rs/) or your system package manager.

_libvlc_ also needs to be available on your system. For most
Linux systems it is sufficient to install VLC through the
package manager. On Mac, make sure VLC is in `/Applications`
and check out _Linker Paths on Mac OS_ in the next section.
Windows is not yet supported, sorry.

For running, `espeak` should be on the path. For Windows and Mac,
there are fallback solutions in place, but installing `espeak`
ensures a consistent user experience.

#### Caveat: Linker Paths on Mac OS
This only applies if Mac OS is the target system.

You need to set some environment variables for executing the generated
binaries in order for them to find VLC. This includes test runs, which
have the same dependencies.

The generated binaries, when loaded, will try to locate VLC relative
to the executable in the directory (`@loader_path/lib`). `@loader_path`
is the containing directory of the binary being executed. I have not
yet found a way to compile the binary in such a way that it automatically
uses the system-provided _libvlc_.

If you fail to provide the linker paths and try to test or run, you
will get errors like these:

    grace:fernspielapparat krachzack$ cargo test
    Compiling fernspielapparat v0.1.0 (/Users/krachzack/Development/fernspielapparat/fernspielapparat)
        Finished dev [unoptimized + debuginfo] target(s) in 2.62s
        Running target/debug/deps/fernspielapparat-c60364b61c7d4b3e
    dyld: Library not loaded: @loader_path/lib/libvlc.5.dylib
    Referenced from: /Users/krachzack/Development/fernspielapparat/fernspielapparat/target/debug/deps/fernspielapparat-c60364b61c7d4b3e
    Reason: image not found
    error: process didn't exit successfully: `/Users/krachzack/Development/fernspielapparat/fernspielapparat/target/debug/deps/fernspielapparat-c60364b61c7d4b3e` (signal: 6, SIGABRT: process abort signal)

You can symlink the libraries from libVLC in `/Applications/VLC.app/Contents/MacOS/` and the plugins as well, but that would be tedious, especially for
testing.

There is another workaround with environment variables. Extend
`DYLD_LIBRARY_PATH` with the directory of the VLC executable and
also specify the VLC plugin directory with `VLC_PLUGIN_PATH`.
The following worked for me, you can add it to your `.bash_profile`
and run `vlc_link_env_vars` every time you are about to run fernspielapparat
on your mac:

    # Set env variable so dynamic loading of VLC is possible on mac.
    # Call this before using `cargo test` or running `fernspielapparat`
    # after it has been compiled.
    function vlc_link_env_vars {
        export DYLD_LIBRARY_PATH="/Applications/VLC.app/Contents/MacOS/lib:$DYLD_LIBRARY_PATH"
        export VLC_PLUGIN_PATH="/Applications/VLC.app/Contents/MacOS/plugins"
    }

When shipping, you can provide the dylib and the plugin directory inside the app bundle, so the env variables are not needed.

### Compiling
`cargo build --release` generates an executable in the
`target/release` directory.

### Installing
`cargo install --path .` installs the runtime globally on your
path.

## Cross-Compiling
You can follow the above steps on your _Raspberry Pi_ target system and
compile it on there, but that will take a long time. If you intend to change
something and compile the runtime a lot of times, setting up a cross-compile
toolchain will probably make you more happy. It is not as difficult as it sounds,
and if you intend to do it, here is some advice.

### Getting a Toolchain for `armhf`
This pre-built toolchain worked well for me on linux-based host systems:
https://github.com/rvagg/rpi-newer-crosstools/blob/master/x64-gcc-6.3.1.config

Provide it on your `PATH` when compiling.

You can also try using a cross-compiler compiler tool like `crosstool-ng`
and build a toolchain yourself.

Have not tried this yet on Mac or Windows, but it should be doable.

### Provide _libvlc_ Shared Libraries
When linking, the shared libraries that will be used on the target system
need to be known to the linker of your cross compile toolchain. The build
configuration is set up so you can provide them in a `vendor/arm-unknown-linux-gnueabihf/vlc/lib` directory next to the top-level
fernspielapparat directory.

You can obtain the libraries by copying them from the target systems or
obtaining libraries of the same version and architecture from the debian
stretch repository, if your Raspberry is running _Raspbian_.

I needed the following libraries to successfully compile, but as time
passes, the set may change:

    libdbus-1.so.3          libpcre.so.3
    libdbus-1.so.3.14.16    libpcre.so.3.13.3
    libgcrypt.so.20         libselinux.so.1
    libgcrypt.so.20.1.6     libsystemd.so.0
    libgpg-error.so.0       libsystemd.so.0.17.0
    libgpg-error.so.0.21.0  libvlccore.so
    libidn.so.11            libvlccore.so.9
    libidn.so.11.6.16       libvlccore.so.9.0.0
    liblz4.so.1             libvlc.so
    liblz4.so.1.7.1         libvlc.so.5
    liblzma.so.5            libvlc.so.5.6.0
    liblzma.so.5.2.2

When in doubt, check which libraries your target system has installed.

### `deploy.sh`
A script is included that combines building and copying the binary to
a Raspberry Pi. It also installs a unit file so the runtime will be
launched at system startup.

For it to work, set up publickey authentication with `ssh-copy-id` so
you can access the Raspberry Pi from a script without a password.

Then, override the following variables in the script:

    RPI_USER="pi"
    RPI_HOSTNAME="your-raspberry-hostname-or-ip"
