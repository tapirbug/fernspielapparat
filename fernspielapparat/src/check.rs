//! Functionality to perform hardware checks without running
//! a phonebook.
use crate::phone::Phone;
use crate::result::Result;

use log::{error, info};
use tavla::{any_voice, Speech, Voice};

use std::thread::sleep;
use std::time::Duration;

/// Checks I2C phone and speech synthesis.
///
/// If any of the two does not stand the check, then
/// an error with more details is returned.
pub fn check_system() -> Result<()> {
    let check_result = check_phone().and(check_speech());

    if check_result.is_ok() {
        info!("Systems check successful.");
    } else {
        error!("Systems check failure.");
    }

    check_result
}

/// Checks if the I2C phone can be connected to and then
/// tries to ring for one second.
///
/// Returns an error if the check did not work out at some
/// point.
pub fn check_phone() -> Result<()> {
    info!("Testing communication with hardware phone...");

    let test_result = Phone::connect("/dev/i2c-1", 4).and_then(|mut phone| {
        phone.ring()?;
        sleep(Duration::from_secs(1));
        phone.unring()?;
        Ok(())
    });

    match test_result {
        Ok(_) => info!("Hardware phone ok."),
        Err(ref e) => {
            error!("Communication with hardware phone failed: {}.", e);
        }
    }

    Ok(test_result?)
}

/// Checks if speech synthesis is working by speaking the
/// sentence "This is fernspielapparat speaking.".
pub fn check_speech() -> Result<()> {
    info!("Testing speech synthesizer...");

    let test_result = any_voice().and_then(|v| {
        Ok(v.speak("This is fernspielapparat speaking.")?
            .await_done()?)
    });

    match test_result {
        Ok(_) => {
            info!("Speech synthesis ok.");
            Ok(())
        }
        Err(e) => {
            error!("Speech synthesis failed: {}.", e);
            Err(e)
        }
    }
}
