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

const TOLERANCE: f32 = 0.0001;
const MOVE_SPEED: f32 = 0.05;
const TURN_SPEED: f32 = 0.02;

enum Tile {
    Empty,
    Solid(Color),
}

/// A tile-based map.
///
/// Top-left is (0, 0).
/// Indexing is (column, row).
///
struct Map {
    tiles: Vec<Vec<Tile>>,
    width: usize,
    height: usize,
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
            if random::<f32>() < 0.05 {
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

    Map {
        tiles: map,
        width,
        height,
    }
}

pub struct Level {
    map: Map,
    player_x: f32,
    player_y: f32,
    player_angle: f32,
}

struct Projection {
    x: f32,
    y: f32,
    color: Color,
}

struct PathIndex {
    row: usize,
    column: usize,
}

fn float_eq(f1: f32, f2: f32) -> bool {
    (f2 - f1).abs() < TOLERANCE
}

impl Level {
    pub fn new(_files: &FileManager, _images: &mut dyn ImageLoader) -> Level {
        Level {
            map: create_random_map(32, 40),
            player_x: 16.5,
            player_y: 20.5,
            player_angle: 0.0,
        }
    }

    fn project(
        &self,
        angle: f32,
        x: f32,
        y: f32,
        path: &mut Option<Vec<PathIndex>>,
    ) -> Option<Projection> {
        let column = x as usize;
        let row = y as usize;
        let x = x - column as f32;
        let y = y - row as f32;
        self.project2(angle, row, column, x, y, path)
    }

    /// Projects a line through the tile map.
    ///
    /// angle: the angle
    fn project2(
        &self,
        angle: f32,
        row: usize,
        column: usize,
        x: f32,
        y: f32,
        path: &mut Option<Vec<PathIndex>>,
    ) -> Option<Projection> {
        // Check out of bounds.
        if row >= self.map.height || column >= self.map.width {
            return None;
        }

        if let Some(path) = path.as_mut() {
            path.push(PathIndex { row, column });
        }

        // Check for collision.
        if let Tile::Solid(color) = self.map.tiles[row][column] {
            return Some(Projection {
                x: column as f32 + x,
                y: row as f32 + y,
                color,
            });
        }

        // Check the cardinal directions, since the math gets funky.
        if float_eq(angle, 0.0) {
            // Straight right.
            return self.project2(angle, row, column + 1, 0.0, y, path);
        }
        if float_eq(angle, PI) {
            // Straight left.
            return self.project2(angle, row, column - 1, 1.0, y, path);
        }
        if float_eq(angle, PI / 2.0) {
            // Straight down.
            return self.project2(angle, row + 1, column, x, 0.0, path);
        }
        if float_eq(angle, (3.0 * PI) / 2.0) {
            // Straight up.
            return self.project2(angle, row - 1, column, x, 1.0, path);
        }

        // TODO: Try to simplify this.

        // Check the odd angles.
        //
        //        0 - PI/2: right and down
        //       PI/2 - PI: left and down
        //     PI - 3 PI/2: left and up
        // 3 PI / 2 - 2 PI: right and up

        if angle < PI {
            // It's pointing downish.
            /*
             *      +------------+
             *      |            |
             *      |        dx  |
             *      |       *--+-|
             *      |  ny-y |\θ| |
             *      |       | \| |
             *      +------------+
             */

            let x_intercept = x + (1.0 - y) / angle.tan();
            if x_intercept <= 0.0 {
                // it hit the left.
                if column == 0 {
                    None
                } else {
                    let y_intercept = 1.0 - ((1.0 - y) + x * angle.tan());
                    self.project2(angle, row, column - 1, 1.0, y_intercept, path)
                }
            } else if x_intercept < 1.0 {
                // it hit the bottom.
                self.project2(angle, row + 1, column, x_intercept, 0.0, path)
            } else {
                // it hit the right.
                let y_intercept = y + (1.0 - x) * angle.tan();
                self.project2(angle, row, column + 1, 0.0, y_intercept, path)
            }
        } else {
            // It's pointing upish.
            /*
             *               dx
             *      +------------+
             *      |       | /  |
             *      |     y |/θ  |
             *      |       *--+-|
             *      |            |
             *      |            |
             *      +------------+
             */
            let up_angle = (2.0 * PI) - angle;
            let x_intercept = x + y / up_angle.tan();
            if x_intercept <= 0.0 {
                // it hit the left.
                if column == 0 {
                    None
                } else {
                    let y_intercept = 1.0 - ((1.0 - y) - x * up_angle.tan());
                    self.project2(angle, row, column - 1, 1.0, y_intercept, path)
                }
            } else if x_intercept < 1.0 {
                // it hit the top.
                self.project2(angle, row - 1, column, x_intercept, 1.0, path)
            } else {
                // it hit the right.
                let y_intercept = y - (1.0 - x) * up_angle.tan();
                self.project2(angle, row, column + 1, 0.0, y_intercept, path)
            }
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
        while self.player_angle >= PI * 2.0 {
            self.player_angle -= PI * 2.0;
        }
        while self.player_angle < 0.0 {
            self.player_angle += PI * 2.0;
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

        // Draw the 2d version.

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

        let player_color = Color::from_str("#7fff0000").unwrap();
        let start_theta = self.player_angle - (PI / 4.0);
        let end_theta = self.player_angle + (PI / 4.0);
        context.player_batch.fill_arc(
            Point {
                x: (self.player_x * w as f32) as i32,
                y: (self.player_y * h as f32) as i32,
            },
            40.0,
            start_theta,
            end_theta,
            player_color,
        );

        // draw the 3d version.
        for column in 0..320 {
            let angle = ((column as f32) / 320.0) * (PI / 2.0);
            let angle = angle - (PI / 4.0);
            let mut angle = self.player_angle + angle;
            while angle >= PI * 2.0 {
                angle -= PI * 2.0;
            }
            while angle < 0.0 {
                angle += PI * 2.0;
            }

            if let Some(projection) = self.project(angle, self.player_x, self.player_y, &mut None) {
                let distance = ((self.player_x - projection.x) * (self.player_x - projection.x)
                    + (self.player_y - projection.y) * (self.player_y - projection.y))
                    .sqrt();

                let scale = if distance < 1.0 { 1.0 } else { 1.0 / distance };
                let height = (RENDER_HEIGHT as f32 * scale) as i32;
                let offset = (RENDER_HEIGHT as i32 - height) / 2;

                context.player_batch.draw_line(
                    Point {
                        x: 320 + column,
                        y: offset,
                    },
                    Point {
                        x: 320 + column,
                        y: offset + height,
                    },
                    projection.color,
                    1,
                );
            }
        }

        // draw a single line point.
        let looking_color = Color::from_str("#FFFFFF").unwrap();
        let mut path = Some(Vec::new());
        let maybe_projection =
            self.project(self.player_angle, self.player_x, self.player_y, &mut path);
        let path_color = Color::from_str("#44ffffff").unwrap();
        for PathIndex { row: i, column: j } in path.unwrap() {
            let y = i as i32 * h;
            let x = j as i32 * w;
            let rect = Rect { x, y, w, h };
            context.player_batch.fill_rect(rect, path_color);
        }
        if let Some(looking_at) = maybe_projection {
            context.player_batch.draw_line(
                Point {
                    x: (w as f32 * self.player_x) as i32,
                    y: (h as f32 * self.player_y) as i32,
                },
                Point {
                    x: (w as f32 * looking_at.x) as i32,
                    y: (h as f32 * looking_at.y) as i32,
                },
                looking_color,
                3,
            );
        }
    }
}
