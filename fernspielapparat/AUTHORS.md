# Authors
This document gives credit to the original authors of assets and
libraries that were used in this project for testing and in production
code.

## Assets
Sound samples used for tests and the embedded demo phonebook.

### Erokia - MSfxp3 Sounds (Pack 3) » MSfxP3 - 15 (Thunky Bass) 
Use: As a short sample for unit, integration and acceptance tests.

Path in project: `test/482381__erokia__msfxp3-15-thunky-bass.wav`

Path in released tarball: n/a

License: Attribution-NonCommercial 3.0 Unported (CC BY-NC 3.0)

Source: Download from [freesound.org](https://freesound.org/people/Erokia/sounds/482381/) on 2019-09-02

### A Good Bass for Gambling
Use: Background music in embedded demo phonebook.

Path in project: `test/A Good Bass for Gambling.mp3`

Path in released tarball: `fernspielapparat` (embedded into binary)

License: CC0

Source: Download from [freemusicarchive.org](http://freemusicarchive.org/music/Komiku/Its_time_for_adventure__vol_3/Komiku_-_Its_time_for_adventure_vol_3_-_06_A_good_bass_for_gambling) on 2019-09-02

## Dynamically Linked Libraries
Dynamically linked libraries.

### VLC
Use: Compiled binary dynamically links to `libvlc` using the `vlc-rs` bindings.

Statically linked: no

### espeak
Use: text-to-speech through its CLI via `tavla` crate

Statically linked: no

## Rust Crates
Statically linked rust crates. For the specific versions in use,
please refer to [Cargo.toml](Cargo.toml).

### base64
Use: Decodes the base64 part of data URIs.

Statically linked: yes

License: MIT/Apache-2.0

Source: crates.io

### clap
Use: Does command line argument parsing.

Statically linked: yes

License: MIT

Source: crates.io

### crossbeam-channel
Use: Asynchroneous, non-blocking, fixed-size channels for communication with background workers.

Statically linked: yes

License: MIT/Apache-2.0 AND BSD-2-Clause

Source: crates.io

### ctrlc
Use: Graceful shutdown by catching SIGTERM and SIGINT.

Statically linked: yes

License: MIT/Apache-2.0

Source: crates.io

### cute-log
Use: Logging frontend used in production builds.

Statically linked: yes

License: Apache-2.0

Source: crates.io

### derivative
Use: Implements standard traits like `Hash`, but can e.g. ignore some fields.

Statically linked: yes

License: MIT/Apache-2.0

Source: crates.io

### failure
Use: Error management and formatting.

Statically linked: yes

License: MIT

Source: crates.io

### log
Use: Logging facade, provides macros for logging.

Statically linked: yes

License: MIT

Source: crates.io

### serde
Use: Base package for serialization and deserialization.

Statically linked: yes

License: MIT

Source: crates.io

### serde_yaml
Use: YAML serialization for serde.

Statically linked: yes

License: MIT

Source: crates.io

### tavla
Use: Rust frontend for the espeak CLI.

Statically linked: yes

License: GPLv3

Source: crates.io

### tempfile
Use: Temporary directories and cleanup for them.

Statically linked: yes

License: MIT

Source: crates.io

### vlc-rs
Use: Bindings to libvlc for rust.

Statically linked: yes

License: MIT

Source: crates.io

### websocket
Use: Websocket functionality used for the remote control server.

Statically linked: yes

License: MIT

Source: crates.io
