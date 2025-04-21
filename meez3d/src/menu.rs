use std::path::Path;

use anyhow::Result;
use log::error;

use crate::cursor::Cursor;
use crate::filemanager::FileManager;
use crate::font::Font;
use crate::geometry::{Point, Rect};
use crate::imagemanager::ImageLoader;
use crate::inputmanager::InputSnapshot;
use crate::rendercontext::{RenderContext, RenderLayer};
use crate::scene::{Scene, SceneResult};
use crate::soundmanager::SoundManager;
use crate::sprite::Sprite;
use crate::uibutton::UiButton;
use crate::utils::Color;
use crate::RENDER_WIDTH;

pub struct Menu {
    cancel_action: String,
    cursor: Cursor,
    background: Sprite,
    buttons: Vec<UiButton>,
    selected: usize,
    text: Option<String>,
}

enum ButtonOrderDirection {
    Vertical,
    Horizontal,
}

impl Menu {
    pub fn new_splash(files: &FileManager, images: &mut dyn ImageLoader) -> Result<Self> {
        let background_path = Path::new("assets/splash.png");
        let cancel_action = "menu";
        let mut menu = Menu::new(background_path, cancel_action, None, files, images)?;
        let start = Rect {
            x: 60,
            y: 80,
            w: 394,
            h: 145,
        };
        menu.add_button(Path::new("assets/start_button.png"), start, "level", images)?;
        Ok(menu)
    }

    pub fn new_kill_screen(
        text: &str,
        files: &FileManager,
        images: &mut dyn ImageLoader,
    ) -> Result<Self> {
        let background_path = Path::new("assets/red.png");
        let cancel_action = "level";
        let text = Some(text.to_string());
        let mut menu = Menu::new(background_path, cancel_action, text, files, images)?;
        let retry = Rect {
            x: 800 - 197,
            y: 450,
            w: 394,
            h: 145,
        };
        let quit = Rect {
            x: 800 - 197,
            y: 650,
            w: 394,
            h: 145,
        };
        menu.add_button(Path::new("assets/retry_button.png"), retry, "level", images)?;
        menu.add_button(Path::new("assets/quit_button.png"), quit, "menu", images)?;
        Ok(menu)
    }

    fn new(
        background_path: &Path,
        cancel_action: &str,
        text: Option<String>,
        _files: &FileManager,
        images: &mut dyn ImageLoader,
    ) -> Result<Self> {
        let cancel_action = cancel_action.to_string();
        let cursor = Cursor::new(images)?;
        let background = images.load_sprite(background_path)?;
        let buttons = Vec::new();
        let selected = 0;

        Ok(Self {
            cancel_action,
            cursor,
            background,
            buttons,
            selected,
            text,
        })
    }

    fn add_button(
        &mut self,
        path: &Path,
        position: Rect<i32>,
        action: &str,
        images: &mut dyn ImageLoader,
    ) -> Result<()> {
        let button = UiButton::new(path, position, action, images)?;
        self.buttons.push(button);
        Ok(())
    }

    fn next_button(&mut self, delta: i32, direction: ButtonOrderDirection) {
        self.selected = (self.selected + 1) % self.buttons.len();
    }

    fn perform_action(&self, action: &str) -> Option<SceneResult> {
        Some(if action == "level" {
            SceneResult::PushLevel
        } else if action == "menu" {
            SceneResult::PushMenu
        } else if action == "pop" {
            SceneResult::Pop
        } else if action == "pop2" {
            SceneResult::PopTwo
        } else if action == "reload" {
            SceneResult::ReloadLevel
        } else {
            error!("invalid button action: {action}");
            return None;
        })
    }
}

impl Scene for Menu {
    fn update(
        &mut self,
        _context: &RenderContext,
        inputs: &InputSnapshot,
        sounds: &mut SoundManager,
    ) -> SceneResult {
        if inputs.cancel_clicked {
            if let Some(result) = self.perform_action(&self.cancel_action) {
                return result;
            }
        }

        if inputs.menu_down_clicked {
            self.next_button(1, ButtonOrderDirection::Vertical);
        }
        if inputs.menu_up_clicked {
            self.next_button(-1, ButtonOrderDirection::Vertical);
        }
        if inputs.menu_left_clicked {
            self.next_button(-1, ButtonOrderDirection::Horizontal);
        }
        if inputs.menu_right_clicked {
            self.next_button(1, ButtonOrderDirection::Horizontal);
        }

        self.cursor.update(inputs);

        let mut clicked_action = None;
        for (i, button) in self.buttons.iter_mut().enumerate() {
            let selected = i == self.selected;
            if let Some(action) = button.update(selected, inputs, sounds) {
                clicked_action = Some(action);
            }
        }
        if let Some(action) = clicked_action {
            if let Some(result) = self.perform_action(&action) {
                return result;
            }
        }

        SceneResult::Continue
    }

    fn draw(&self, context: &mut RenderContext, font: &Font, previous: Option<&dyn Scene>) {
        context.player_batch.fill_rect(
            context.logical_area(),
            Color {
                r: 0x33,
                g: 0x00,
                b: 0x33,
                a: 0xff,
            },
        );

        if let Some(background) = previous {
            background.draw(context, font, None);
        }

        let src = Rect {
            x: 0,
            y: 0,
            w: 1600,
            h: 900,
        };
        context
            .hud_batch
            .draw(self.background, context.logical_area(), src, false);

        if let Some(text) = self.text.as_ref() {
            let text_width = text.len() as i32 * font.char_width;
            let text_pos = Point::new((RENDER_WIDTH as i32 - text_width) / 2, 250);
            font.draw_string(context, RenderLayer::Hud, text_pos, text);
        }

        for button in self.buttons.iter() {
            button.draw(context, RenderLayer::Hud, font);
        }
        self.cursor.draw(context, RenderLayer::Hud);
    }
}
