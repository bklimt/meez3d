use crate::constants::{RENDER_HEIGHT, RENDER_WIDTH};
use crate::filemanager::FileManager;
use crate::geometry::{Point, Rect};
use crate::imagemanager::ImageLoader;
use crate::inputmanager::InputSnapshot;
use crate::scene::Scene;
use crate::scene::SceneResult;
use crate::utils::Color;
use crate::RenderContext;
use crate::SoundManager;
use crate::{Font, FRAME_RATE};
use rand::random;
use std::f32::consts::PI;
use std::str::FromStr;

const MOVE_SPEED: f32 = 0.05;
const TURN_SPEED: f32 = 0.02;

enum Tile {
    Empty,
    Solid(Color),
}

struct Map {
    tiles: Vec<Vec<Tile>>,
}

fn uniform_random(min: f32, max: f32) -> f32 {
    let range = max - min;
    min + random::<f32>() * range
}

fn create_random_row(width: usize) -> Vec<Tile> {
    let mut row = Vec::new();
    let border_color = Color::from_str("#0000ff").unwrap();
    row.push(Tile::Solid(border_color));
    row.extend(
        std::iter::repeat_with(|| {
            if random::<f32>() < 0.1 {
                let r = uniform_random(0.0, 256.0) as u8;
                let g = uniform_random(0.0, 256.0) as u8;
                let b = uniform_random(0.0, 256.0) as u8;
                let a = 255;
                let color = Color { r, g, b, a };
                Tile::Solid(color)
            } else {
                Tile::Empty
            }
        })
        .take(width - 2),
    );
    row.push(Tile::Solid(border_color));
    row
}

fn create_random_map(width: usize, height: usize) -> Map {
    let border_color = Color::from_str("#0000ff").unwrap();
    let full_row = || {
        std::iter::repeat_with(|| Tile::Solid(border_color))
            .take(width)
            .collect()
    };

    let mut map = Vec::new();
    map.push(full_row());
    map.extend(std::iter::repeat_with(|| create_random_row(width)).take(height - 2));
    map.push(full_row());

    Map { tiles: map }
}

pub struct Level {
    map: Map,
    player_x: f32,
    player_y: f32,
    player_angle: f32,
}

impl Level {
    pub fn new(_files: &FileManager, _images: &mut dyn ImageLoader) -> Level {
        Level {
            map: create_random_map(32, 40),
            player_x: 2.0,
            player_y: 2.0,
            player_angle: 0.0,
        }
    }
}

impl Scene for Level {
    fn update(
        &mut self,
        context: &RenderContext,
        inputs: &InputSnapshot,
        sounds: &mut SoundManager,
    ) -> SceneResult {
        if inputs.ok_clicked {
            return SceneResult::PushKillScreen {
                text: format!("hello world"),
            };
        }

        if inputs.player_turn_left_down {
            self.player_angle -= TURN_SPEED;
        }
        if inputs.player_turn_right_down {
            self.player_angle += TURN_SPEED;
        }

        let x_component = self.player_angle.cos();
        let y_component = self.player_angle.sin();
        if inputs.player_forward_down {
            self.player_x += MOVE_SPEED * x_component;
            self.player_y += MOVE_SPEED * y_component;
        }
        if inputs.player_backward_down {
            self.player_x -= MOVE_SPEED * x_component;
            self.player_y -= MOVE_SPEED * y_component;
        }
        if inputs.player_strafe_left_down {
            self.player_x += MOVE_SPEED * y_component;
            self.player_y -= MOVE_SPEED * x_component;
        }
        if inputs.player_strafe_right_down {
            self.player_x -= MOVE_SPEED * y_component;
            self.player_y += MOVE_SPEED * x_component;
        }

        SceneResult::Continue
    }

    fn draw(&self, context: &mut RenderContext, font: &Font, previous: Option<&dyn Scene>) {
        let screen = Rect {
            x: 0,
            y: 0,
            w: RENDER_WIDTH as i32,
            h: RENDER_HEIGHT as i32,
        };
        let bgcolor = Color::from_str("#00333c").unwrap();
        context.player_batch.fill_rect(screen, bgcolor);

        let w = 10;
        let h = 10;
        let empty_color = Color::from_str("#000000").unwrap();
        for (i, row) in self.map.tiles.iter().enumerate() {
            let y = i as i32 * h;
            for (j, tile) in row.iter().enumerate() {
                let x = j as i32 * w;
                let rect = Rect { x, y, w, h };
                let color = match tile {
                    Tile::Empty => &empty_color,
                    Tile::Solid(color) => color,
                };
                context.player_batch.fill_rect(rect, *color);
            }
        }

        let player_color = Color::from_str("#ffffff").unwrap();
        context.player_batch.fill_circle(
            Point {
                x: (self.player_x * w as f32) as i32,
                y: (self.player_y * h as f32) as i32,
            },
            5.0,
            player_color,
        );

        let player_color = Color::from_str("#44ff0000").unwrap();
        let start_theta = self.player_angle - (PI / 4.0);
        let end_theta = self.player_angle + (PI / 4.0);
        context.player_batch.fill_arc(
            Point {
                x: (self.player_x * w as f32) as i32,
                y: (self.player_y * h as f32) as i32,
            },
            20.0,
            start_theta,
            end_theta,
            player_color,
        );
    }
}
