use std::f32::consts::PI;

use anyhow::Result;
use log::warn;

use crate::constants::{CIRCLE_STEPS, MAX_LIGHTS};
use crate::geometry::{Point, Rect};
use crate::sprite::Sprite;
use crate::utils::Color;

pub enum SpriteBatchEntry {
    Sprite {
        sprite: Sprite,
        source: Rect<i32>,
        destination: Rect<i32>,
        reversed: bool,
    },
    FillRect {
        destination: Rect<i32>,
        color: Color,
    },
    FillTriangle {
        p1: Point<i32>,
        p2: Point<i32>,
        p3: Point<i32>,
        color: Color,
    },
    Line {
        start: Point<i32>,
        end: Point<i32>,
        color: Color,
        width: i32,
    },
}

pub struct SpriteBatch {
    pub clear_color: Color,
    pub entries: Vec<SpriteBatchEntry>,
}

impl SpriteBatch {
    #[allow(clippy::new_without_default)]
    pub fn new() -> SpriteBatch {
        SpriteBatch {
            clear_color: Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            entries: Vec::new(),
        }
    }

    pub fn draw(&mut self, sprite: Sprite, dst: Rect<i32>, src: Rect<i32>, reversed: bool) {
        self.entries.push(SpriteBatchEntry::Sprite {
            sprite,
            source: src,
            destination: dst,
            reversed,
        });
    }

    pub fn fill_rect(&mut self, rect: Rect<i32>, color: Color) {
        self.entries.push(SpriteBatchEntry::FillRect {
            destination: rect,
            color,
        });
    }

    pub fn fill_triangle(&mut self, p1: Point<i32>, p2: Point<i32>, p3: Point<i32>, color: Color) {
        self.entries
            .push(SpriteBatchEntry::FillTriangle { p1, p2, p3, color });
    }

    pub fn draw_line(&mut self, point1: Point<i32>, point2: Point<i32>, color: Color, width: i32) {
        if point1.y == point2.y {
            // horizontal
            let rect = if point1.x > point2.x {
                Rect {
                    x: point2.x,
                    y: point2.y - width / 2,
                    w: point1.x - point2.x,
                    h: width,
                }
            } else {
                Rect {
                    x: point1.x,
                    y: point1.y - width / 2,
                    w: point2.x - point1.x,
                    h: width,
                }
            };
            self.fill_rect(rect, color);
        } else if point1.x == point2.x {
            // vertical
            let rect = if point1.y > point2.y {
                Rect {
                    x: point2.x - width / 2,
                    y: point2.y,
                    w: width,
                    h: point1.y - point2.y,
                }
            } else {
                Rect {
                    x: point1.x - width / 2,
                    y: point1.y,
                    w: width,
                    h: point2.y - point1.y,
                }
            };
            self.fill_rect(rect, color);
        } else {
            self.entries.push(SpriteBatchEntry::Line {
                start: point1,
                end: point2,
                color,
                width,
            })
        }
    }

    pub fn fill_circle(&mut self, center: Point<i32>, radius: f32, color: Color) {
        self.fill_arc(center, radius, 0.0, 2.0 * PI, color);
    }

    pub fn fill_arc(
        &mut self,
        center: Point<i32>,
        radius: f32,
        start_theta: f32,
        end_theta: f32,
        color: Color,
    ) {
        let mut theta: f32 = start_theta;
        let mut current = Point::new(theta.cos(), theta.sin());
        let dtheta = (2.0 * PI) / (CIRCLE_STEPS as f32);
        // TODO: Make the arc actually end at the right spot.
        while theta <= end_theta {
            let next_theta = theta + dtheta;
            let next = Point::new(next_theta.cos(), next_theta.sin());

            let p1 = current * radius;
            let p2 = next * radius;
            let p1 = Point::new(p1.x as i32, p1.y as i32);
            let p2 = Point::new(p2.x as i32, p2.y as i32);
            let p1 = p1 + center;
            let p2 = p2 + center;

            self.fill_triangle(center, p2, p1, color);

            current = next;
            theta = next_theta;
        }
    }

    pub fn draw_circle(&mut self, center: Point<i32>, radius: f32, color: Color, width: i32) {
        let mut theta: f32 = 0.0;
        let mut current = Point::new(theta.cos(), theta.sin());
        let dtheta = (2.0 * PI) / (CIRCLE_STEPS as f32);
        while theta <= 2.0 * PI {
            let next_theta = theta + dtheta;
            let next = Point::new(next_theta.cos(), next_theta.sin());

            let p1 = current * radius;
            let p2 = next * radius;
            let p1 = Point::new(p1.x as i32, p1.y as i32);
            let p2 = Point::new(p2.x as i32, p2.y as i32);
            let p1 = p1 + center;
            let p2 = p2 + center;

            self.draw_line(p1, p2, color, width);

            current = next;
            theta = next_theta;
        }
    }
}

pub struct Light {
    pub position: Point<i32>,
    pub radius: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum RenderLayer {
    Player,
    Hud,
}

pub struct RenderContext {
    pub player_batch: SpriteBatch,
    pub hud_batch: SpriteBatch,
    pub width: u32,
    pub height: u32,
    pub frame: u64,
    pub lights: Vec<Light>,
    pub is_dark: bool,
}

impl RenderContext {
    pub fn new(width: u32, height: u32, frame: u64) -> Result<RenderContext> {
        let player_batch = SpriteBatch::new();
        let hud_batch = SpriteBatch::new();
        let lights = Vec::new();
        let is_dark = false;
        Ok(RenderContext {
            player_batch,
            hud_batch,
            width,
            height,
            frame,
            lights,
            is_dark,
        })
    }

    pub fn logical_area(&self) -> Rect<i32> {
        // TODO: This should be cacheable.
        Rect {
            x: 0,
            y: 0,
            w: self.width as i32,
            h: self.height as i32,
        }
    }

    pub fn draw(&mut self, sprite: Sprite, layer: RenderLayer, dst: Rect<i32>, src: Rect<i32>) {
        match layer {
            RenderLayer::Player => self.player_batch.draw(sprite, dst, src, false),
            RenderLayer::Hud => self.hud_batch.draw(sprite, dst, src, false),
        }
    }

    pub fn draw_reversed(
        &mut self,
        sprite: Sprite,
        layer: RenderLayer,
        dst: Rect<i32>,
        src: Rect<i32>,
    ) {
        match layer {
            RenderLayer::Player => self.player_batch.draw(sprite, dst, src, true),
            RenderLayer::Hud => self.hud_batch.draw(sprite, dst, src, true),
        }
    }

    pub fn fill_rect(&mut self, rect: Rect<i32>, layer: RenderLayer, color: Color) {
        match layer {
            RenderLayer::Player => self.player_batch.fill_rect(rect, color),
            RenderLayer::Hud => self.hud_batch.fill_rect(rect, color),
        }
    }

    pub fn clear(&mut self) {
        self.player_batch.entries.clear();
        self.hud_batch.entries.clear();
        self.player_batch.clear_color = Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        };
        self.hud_batch.clear_color = Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }

    pub fn add_light(&mut self, position: Point<i32>, radius: i32) {
        if self.lights.len() >= MAX_LIGHTS {
            warn!("too many lights set");
            return;
        }
        self.lights.push(Light { position, radius });
    }
}
