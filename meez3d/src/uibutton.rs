use std::path::Path;

use anyhow::Result;
use log::info;

use crate::font::Font;
use crate::geometry::Point;
use crate::geometry::Rect;
use crate::imagemanager::ImageLoader;
use crate::inputmanager::InputSnapshot;
use crate::rendercontext::RenderContext;
use crate::rendercontext::RenderLayer;
use crate::soundmanager::Sound;
use crate::soundmanager::SoundManager;
use crate::sprite::Sprite;

#[derive(Debug, Clone, Copy)]
enum UiButtonState {
    Normal = 0,
    Hover = 1,
    MouseClick = 2,
    GamepadClick = 3,
}

pub struct UiButton {
    pub position: Rect<i32>,
    sprite: Sprite,
    state: UiButtonState,
    action: String,
}

impl UiButton {
    pub fn new(
        sprite_path: &Path,
        position: Rect<i32>,
        action: &str,
        images: &mut dyn ImageLoader,
    ) -> Result<Self> {
        let sprite = images.load_sprite(sprite_path)?;
        let state = UiButtonState::Normal;
        let action = action.to_string();
        Ok(UiButton {
            position,
            sprite,
            state,
            action,
        })
    }

    pub fn update(
        &mut self,
        selected: bool,
        inputs: &InputSnapshot,
        sounds: &mut SoundManager,
    ) -> Option<String> {
        let mut clicked = false;
        let mouse_inside = self.position.contains(inputs.mouse_position.into());

        self.state = if matches!(self.state, UiButtonState::MouseClick) {
            if inputs.mouse_button_left_down {
                self.state
            } else {
                if mouse_inside {
                    info!("uibutton clicked");
                    clicked = true;
                }
                UiButtonState::Normal
            }
        } else if matches!(self.state, UiButtonState::GamepadClick) {
            if inputs.ok_down {
                self.state
            } else {
                info!("uibutton clicked");
                clicked = true;
                UiButtonState::Normal
            }
        } else if selected && inputs.ok_down {
            UiButtonState::GamepadClick
        } else if mouse_inside && inputs.mouse_button_left_down {
            UiButtonState::MouseClick
        } else if selected || mouse_inside {
            UiButtonState::Hover
        } else {
            UiButtonState::Normal
        };

        if clicked {
            sounds.play(Sound::Click);
            Some(self.action.clone())
        } else {
            None
        }
    }

    pub fn draw(&self, context: &mut RenderContext, layer: RenderLayer, font: &Font) {
        let src = Rect {
            x: 0,
            y: 0,
            w: self.sprite.area.w,
            h: self.sprite.area.h,
        };
        let dst = if matches!(
            self.state,
            UiButtonState::MouseClick | UiButtonState::GamepadClick
        ) {
            self.position + Point::new(16, 16)
        } else {
            self.position
        };
        context.draw(self.sprite, layer, dst, src);
    }
}
