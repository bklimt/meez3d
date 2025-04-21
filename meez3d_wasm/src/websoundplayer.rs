use std::path::Path;

use base64::prelude::*;

use anyhow::{anyhow, Result};
use log::error;
use meez3d::{FileManager, Sound, SoundPlayer};
use web_sys::HtmlAudioElement;

pub struct WebSoundPlayer {
    click_sound: HtmlAudioElement,
}

fn load_image(path: &Path, files: &FileManager) -> Result<HtmlAudioElement> {
    let bytes = files.read(path)?;
    let base64 = BASE64_STANDARD.encode(bytes);
    let url = format!("data:audio/wav;base64,{}", base64);
    let element = HtmlAudioElement::new_with_src(&url)
        .map_err(|e| anyhow!("error creating html audio element: {:?}", e))?;
    Ok(element)
}

impl WebSoundPlayer {
    pub fn new(files: &FileManager) -> Result<Self> {
        let click_sound = load_image(Path::new("assets/sounds/click.wav"), files)?;
        Ok(Self { click_sound })
    }
}

impl SoundPlayer for WebSoundPlayer {
    fn play(&mut self, sound: Sound) {
        if let Err(e) = match sound {
            Sound::Click => self.click_sound.play(),
        } {
            error!("unable to play sound: {:?}", e);
        }
    }
}
