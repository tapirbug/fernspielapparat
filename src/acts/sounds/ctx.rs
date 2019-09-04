use crate::result::Result;

use failure::format_err;

/// Manages resources required for creating players.
pub struct PlayerContext(vlc::Instance);

impl PlayerContext {
    pub fn new() -> Result<Self> {
        vlc::Instance::new()
            .ok_or_else(|| format_err!("Could not load libvlc"))
            .map(PlayerContext)
    }

    pub(crate) fn vlc_instance(&self) -> &vlc::Instance {
        &self.0
    }
}
