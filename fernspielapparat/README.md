# fernspielapparat
This is the fernspielapparat runtime. It is the component running
on the target device that enables users to play phonebooks.

General information on the fernspielapparat project is available
in the [Introduction](https://github.com/krachzack/fernspielapparat/blob/master/doc/Introduction.md).

The runtime loads and evaluates phonebooks. For this, it accesses
hardware dials through an I2C protocol. System keyboard input is
supported on systems without a dial installed. It provides speech
output through _espeak_ and _libvlc_. An optional hardware bell
is supported through an I2C protocol.

## Installing
The runtime runs on Raspberry Pi 2 or similar systems and also on
common desktop operating systems.

Binary releases contain the runtime as an executable file
`fernspielapparat` (or `fernspielapparat.exe`). You can run
it directly and move it anywhere you like to install it.

To make `fernspielapparat` available globally, you can add the
containing directory to your `PATH` environment variable or
move the runtime to some directory that you know is on the
`PATH`, e.g. `/usr/bin` on many systems. Instead of moving,
you can place a symbolic link in the target directory - this
will make it easier for you to switch between versions later
on.

To build and install the runtime yourself, see the
[compilation guide](COMPILE.md).

## Running
Once the runtime is installed, you can run phonebooks with
`fernspielapparat your_phonebook_here.yaml`.

`fernspielapparat --demo` can be used instead of specifying
a file and loads a demo phonebook embedded in the runtime
executable. It contains speech synthesis and background music
appears after some seconds.

`fernspiealapparat --test` starts diagnostic mode. It will
try to ring the bell for one second and access speech
synthesis.

Use `fernspielapparat --help` for an overview of available
options.

## License
The fernspielapparat project is licensed under the GPLv3, since it
internally depends on espeak, which uses that license.
