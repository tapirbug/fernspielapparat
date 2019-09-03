# Attributions
This document gives credit to the original authors of assets and
libraries that were used in this project for testing and to build
the release tarballs.

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

## Libraries
Dynamically linked libraries. For used crates please refer to [Cargo.toml](Cargo.toml).

### VLC
Use: Compiled binary dynamically links to `libvlc` using the `vlc-rs` bindings.

Statically linked: no

### espeak
Use: text-to-speech through its CLI via `tavla` crate

Statically linked: no