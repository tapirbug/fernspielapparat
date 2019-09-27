# Compiling the fernspiealapparat Runtime
This guide is supposed to help you compile the fernspielapparat
runtime for your host system or by using cross-compilation to
compile it for deployment on a Raspberry Pi 2 device.

## Compile for Your Own Computer
The following steps enable you to compile the runtime locally.

### Get the Source Code
If you have not done so already, clone the source code repository:

    git clone https://github.com/krachzack/fernspielapparat.git

### Build System
The rust build system cargo is required. Download it with
[rustup](https://rustup.rs/) or your system package manager.

_libvlc_ also needs to be available on your system. For most
Linux systems it is sufficient to install VLC through the
package manager. On Mac, make sure VLC is in `/Applications`
and check out _Linker Paths on Mac OS_ in the next section.
Windows requires some extra setup, sorry, see below.

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

### Caveat: Getting `libvlc.dll` and `vlc.lib` on Windows
This only applies if you are building for Windows.

Install VLC through their web site if you have not done
so already. Then, make sure the VLC installation directory
is on the `Path` environment variable, so the runtime
can find it. On my system, `libvlc.dll` is located in
`C:\Program Files\VideoLAN\VLC`. Check yourself and
then add it to `Path` through system settings.

On a german locale, progress through the screens like
this after you have found "Systemumgebungsvariablen bearbeiten"
in the system settings, adding your systems path:

![setting Path on Windows to find libvlc.dll](doc/vlc-path-windows.png)

If this was too much German for you, there are a lot of
guides on the internet for setting environment variables
on Windows.

Since `vlc.lib` does not ship with VLC, it has to be either generated
or you have to build VLC yourself. Place it next to the DLL, so it
can be used for linking the executable.

There is a great [guide](https://wiki.videolan.org/GenerateLibFromDll/)
in the VLC wiki that describes how to generate it using Visual Studio
command line tools from an existing VLC installation. Use the developer
command line prompt from your Visual Studio installation to run the commands
described there. Take care to use `x64` instead of the `x86` in the guide.

To build, I recommend using the `stable-x86_64-pc-windows-gnu` toolchain
that you can obtain with `rustup`. You need to get `gcc` on your path
for it to work. You can get it through some channel such as MinGW or CygWin.

`stable-x86_64-pc-windows-msvc` gave
me some undefined references to `vsnprintf` when linking to VLC, though
I assume this could somehow be fixed if you need to use that toolchain.

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

### `build.sh`
Builds release binaries and packages them into a versioned `.tar.gz`
archive along with documentation and some metadata. The resulting
archive is placed in the release directory and includes version,
target operating system and architecture into its name. For example,
a 64bit Windows 10 system gives me:
`release/fernspielapparat-0.1.0-msys_nt-10.0-x86_64.tar.gz`.

### `deploy.sh`
A script is included that combines building and copying the binary to
a Raspberry Pi. It also installs a unit file so the runtime will be
launched at system startup.

For it to work, set up publickey authentication with `ssh-copy-id` so
you can access the Raspberry Pi from a script without a password.

Then, override the following variables in the script:

    RPI_USER="pi"
    RPI_HOSTNAME="your-raspberry-hostname-or-ip"
