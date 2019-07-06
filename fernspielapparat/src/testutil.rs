use env_logger;

use std::sync::Once;

pub const TEST_MUSIC: &str = "test/A Good Bass for Gambling.mp3";

static INIT: Once = Once::new();

pub fn enable_logging() {
    INIT.call_once(|| {
        let _ = env_logger::builder().is_test(true).init();
    })
}
