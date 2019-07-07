
#[cfg(target_os = "macos")]
const VLC_LIB_DIR : &str = "/Applications/VLC.app/Contents/MacOS/lib";

fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=dylib=vlc");
        println!("cargo:rustc-link-search=native={}", VLC_LIB_DIR);
    }
}
