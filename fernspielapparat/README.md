# fernspielapparat
This is the source code for the fernspielapparat runtime. It is
the component running on the target device that enables users to
play phonebooks.

General information on the fernspielapparat project is available
in the [top-level readme](../README.md).

The runtime loads and evaluates phonebooks. For this, it accesses
hardware dials through an I2C protocol. System keyboard input is
supported on systems without a dial installed. It provides speech
output through _espeak_ and _libvlc_. An optional hardware bell
is supported through an I2C protocol.

## Installing
The runtime runs on Raspberry Pi 2 or similar systems and also on
common desktop operating systems.

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


