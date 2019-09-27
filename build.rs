#[cfg(target_os = "macos")]
const VLC_LIB_DIR: &str = "/Applications/VLC.app/Contents/MacOS/lib";
#[cfg(target_os = "windows")]
const VLC_LIB_DIR: &str = "C:\\Program Files\\VideoLAN\\VLC";

fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=dylib=vlc");
        println!("cargo:rustc-link-search=native={}", VLC_LIB_DIR);
    }
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=dylib=vlc");
        println!("cargo:rustc-link-search=native={}", VLC_LIB_DIR);
    }
}
