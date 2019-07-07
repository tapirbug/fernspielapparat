# Target System: Any
The target system needs to provide some software, including:
* vlc
* espeak

# Target System: Raspberry Pi 2
When vlc is compiled, you should be able to compile it on Raspberry, but it will take a long time.
I recommend setting up cross-compilin.

## Host System for Cross Compilation: Linux
This pre-built toolchain worked well for me: https://github.com/rvagg/rpi-newer-crosstools/blob/master/x64-gcc-6.3.1.config

You can also try crosstool-ng yourself.

# Target System: Mac
You need to set some environment variables when running including test runs, otherwise the dynamic
loader will try to load VLC relative to the executable directory (`@loader_path/lib`). Instead, provide a
`DYLD_LIBRARY_PATH` and also specify the VLC plugin directory with `VLC_PLUGIN_PATH`. The following worked
for me:

    # Set env variable so dynamic loading of VLC is possible on mac.
    # Call this before using `cargo test` or running `fernspielapparat`
    # after it has been compiled.
    function vlc_link_env_vars {
        export DYLD_LIBRARY_PATH="/Applications/VLC.app/Contents/MacOS/lib:$DYLD_LIBRARY_PATH"
        export VLC_PLUGIN_PATH="/Applications/VLC.app/Contents/MacOS/plugins"
    }

If you fail to provide the linker paths, I get errors like these:

    grace:fernspielapparat krachzack$ cargo test
    Compiling fernspielapparat v0.1.0 (/Users/krachzack/Development/fernspielapparat/fernspielapparat)
        Finished dev [unoptimized + debuginfo] target(s) in 2.62s
        Running target/debug/deps/fernspielapparat-c60364b61c7d4b3e
    dyld: Library not loaded: @loader_path/lib/libvlc.5.dylib
    Referenced from: /Users/krachzack/Development/fernspielapparat/fernspielapparat/target/debug/deps/fernspielapparat-c60364b61c7d4b3e
    Reason: image not found
    error: process didn't exit successfully: `/Users/krachzack/Development/fernspielapparat/fernspielapparat/target/debug/deps/fernspielapparat-c60364b61c7d4b3e` (signal: 6, SIGABRT: process abort signal)

When shipping, you can provide the dylib and the plugin directory inside the app bundle, so the
env variables are not needed.

# Caveats on Windows
Have not tried this yet but my guess is that it won't be easy.
