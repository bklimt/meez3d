use std::{mem, path::Path};

use anyhow::Result;

use crate::{
    filemanager::FileManager,
    font::Font,
    imagemanager::ImageLoader,
    inputmanager::InputSnapshot,
    level::Level,
    menu::Menu,
    rendercontext::RenderContext,
    scene::{Scene, SceneResult},
    soundmanager::SoundManager,
};

pub struct StageManager {
    current: Box<dyn Scene>,
    stack: Vec<Box<dyn Scene>>,
}

impl StageManager {
    pub fn new(file_manager: &FileManager, images: &mut dyn ImageLoader) -> Result<StageManager> {
        // let path = Path::new("assets/menus/start.tmx");
        // let splash = Menu::new_splash(file_manager, images)?;
        let level = Level::new(file_manager, images)?;
        Ok(StageManager {
            current: Box::new(level),
            stack: Vec::new(),
        })
    }

    pub fn update(
        &mut self,
        context: &RenderContext,
        inputs: &InputSnapshot,
        files: &FileManager,
        images: &mut dyn ImageLoader,
        sounds: &mut SoundManager,
    ) -> Result<bool> {
        let result = self.current.update(context, inputs, sounds);
        Ok(match result {
            SceneResult::Continue => true,
            SceneResult::Pop => {
                if let Some(next) = self.stack.pop() {
                    self.current = next;
                    true
                } else {
                    false
                }
            }
            SceneResult::PopTwo => {
                self.stack.pop();
                if let Some(next) = self.stack.pop() {
                    self.current = next;
                    true
                } else {
                    false
                }
            }
            SceneResult::PushLevel => {
                let level = Level::new(files, images)?;
                let level = Box::new(level);
                let previous = mem::replace(&mut self.current, level);
                self.stack.push(previous);
                true
            }
            SceneResult::ReloadLevel => {
                self.stack.pop();
                self.current = Box::new(Level::new(files, images)?);
                true
            }
            SceneResult::PushMenu => {
                let menu = Menu::new_splash(files, images)?;
                let menu = Box::new(menu);
                let previous = mem::replace(&mut self.current, menu);
                self.stack.push(previous);
                true
            }
            SceneResult::PushKillScreen { text } => {
                let kill_screen = Menu::new_kill_screen(&text, files, images)?;
                let kill_screen = Box::new(kill_screen);
                let previous = mem::replace(&mut self.current, kill_screen);
                self.stack.push(previous);
                true
            }
            SceneResult::PushPause => {
                let pause_screen = Menu::new_splash(files, images)?;
                let pause_screen = Box::new(pause_screen);
                let previous = mem::replace(&mut self.current, pause_screen);
                self.stack.push(previous);
                true
            }
        })
    }

    pub fn draw(&mut self, context: &mut RenderContext, font: &Font) {
        self.current
            .draw(context, font, self.stack.last().map(Box::as_ref));
    }
}
