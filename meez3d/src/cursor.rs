use std::path::Path;

use anyhow::Result;
use num_traits::Zero;

use crate::geometry::{Point, Rect};
use crate::imagemanager::ImageLoader;
use crate::inputmanager::InputSnapshot;
use crate::rendercontext::{RenderContext, RenderLayer};
use crate::sprite::Sprite;

pub struct Cursor {
    position: Point<i32>,
    sprite: Sprite,
}

impl Cursor {
    pub fn new(images: &mut dyn ImageLoader) -> Result<Self> {
        let position = Point::zero();
        let sprite = images.load_sprite(Path::new("assets/cursor.png"))?;
        Ok(Cursor { position, sprite })
    }

    pub fn draw(&self, context: &mut RenderContext, layer: RenderLayer) {
        let src = Rect {
            x: 0,
            y: 0,
            w: 64,
            h: 64,
        };

        let dest = src + self.position;

        context.draw(self.sprite, layer, dest, src);
    }

    pub fn update(&mut self, input: &InputSnapshot) {
        self.position = input.mouse_position;
    }
}
