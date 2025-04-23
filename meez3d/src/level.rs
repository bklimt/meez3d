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
use std::f32::consts::FRAC_PI_2;
use std::f32::consts::PI;
use std::f32::consts::TAU;
use std::str::FromStr;

const TOLERANCE: f32 = 0.0001;
const PLAYER_SIZE: f32 = 0.8;
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

fn create_random_row(width: usize, border_color: Color) -> Vec<Tile> {
    let mut row = Vec::new();
    row.push(Tile::Solid(border_color));
    row.extend(
        std::iter::repeat_with(|| {
            if random::<f32>() < 0.025 {
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
    let border_color = Color::from_str("#ffffff").unwrap();
    let full_row = || {
        std::iter::repeat_with(|| Tile::Solid(border_color))
            .take(width)
            .collect()
    };

    let mut map = Vec::new();
    map.push(full_row());
    map.extend(std::iter::repeat_with(|| create_random_row(width, border_color)).take(height - 2));
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
    normal: f32,
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

    #[allow(clippy::collapsible_if)]
    fn can_move_to(&self, x: f32, y: f32) -> bool {
        let lower_bound = PLAYER_SIZE / 2.0;
        let upper_bound = 1.0 - (PLAYER_SIZE / 2.0);

        let row = y as usize;
        let col = x as usize;
        let x_frac = x - col as f32;
        let y_frac = y - row as f32;
        if !matches!(self.map.tiles[row][col], Tile::Empty) {
            return false;
        }
        if x_frac < lower_bound {
            if col == 0 || !matches!(self.map.tiles[row][col - 1], Tile::Empty) {
                return false;
            }
        }
        if y_frac < lower_bound {
            if row == 0 || !matches!(self.map.tiles[row - 1][col], Tile::Empty) {
                return false;
            }
        }
        if x_frac > upper_bound {
            if col >= self.map.width - 1 || !matches!(self.map.tiles[row][col + 1], Tile::Empty) {
                return false;
            }
        }
        if y_frac > upper_bound {
            if row >= self.map.height - 1 || !matches!(self.map.tiles[row + 1][col], Tile::Empty) {
                return false;
            }
        }
        true
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
        self.project2(angle, row, column, x, y, -angle, path)
    }

    /// Projects a line through the tile map.
    ///
    /// angle: the angle, with 0 being right, and positive being clockwise, in radians
    /// row: the row of the map the user is in, where 0 is the top
    /// column: the column of the map the user is in
    /// x: where in the tile the user is, in the range [0.0, 1.0]
    /// y: where in the tile the user is, in the range [0.0, 1.0], with 0 being the top
    /// normal: the normal angle of the last cell boundary crossed, defined like angle
    ///
    #[allow(clippy::too_many_arguments)]
    fn project2(
        &self,
        angle: f32,
        row: usize,
        column: usize,
        x: f32,
        y: f32,
        normal: f32,
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
                normal,
            });
        }

        // Check the cardinal directions, since the math gets funky.
        if float_eq(angle, 0.0) {
            // Straight right.
            return self.project2(angle, row, column + 1, 0.0, y, PI, path);
        }
        if float_eq(angle, PI) {
            // Straight left.
            return if column == 0 {
                None
            } else {
                return self.project2(angle, row, column - 1, 1.0, y, 0.0, path);
            };
        }
        if float_eq(angle, FRAC_PI_2) {
            // Straight down.
            return self.project2(angle, row + 1, column, x, 0.0, 3.0 * FRAC_PI_2, path);
        }
        if float_eq(angle, 3.0 * FRAC_PI_2) {
            // Straight up.
            return if row == 0 {
                None
            } else {
                self.project2(angle, row - 1, column, x, 1.0, FRAC_PI_2, path)
            };
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
            if x_intercept < 0.0 {
                // it hit the left.
                if column == 0 {
                    None
                } else {
                    let y_intercept = 1.0 - ((1.0 - y) + x * angle.tan());
                    self.project2(angle, row, column - 1, 1.0, y_intercept, 0.0, path)
                }
            } else if x_intercept < 1.0 {
                // it hit the bottom.
                self.project2(
                    angle,
                    row + 1,
                    column,
                    x_intercept,
                    0.0,
                    3.0 * FRAC_PI_2,
                    path,
                )
            } else {
                // it hit the right.
                let y_intercept = y + (1.0 - x) * angle.tan();
                self.project2(angle, row, column + 1, 0.0, y_intercept, PI, path)
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
            let up_angle = TAU - angle;
            let x_intercept = x + y / up_angle.tan();
            if x_intercept < 0.0 {
                // it hit the left.
                if column == 0 {
                    None
                } else {
                    let y_intercept = 1.0 - ((1.0 - y) - x * up_angle.tan());
                    self.project2(angle, row, column - 1, 1.0, y_intercept, 0.0, path)
                }
            } else if x_intercept < 1.0 {
                // it hit the top.
                if row == 0 {
                    None
                } else {
                    self.project2(angle, row - 1, column, x_intercept, 1.0, FRAC_PI_2, path)
                }
            } else {
                // it hit the right.
                let y_intercept = y - (1.0 - x) * up_angle.tan();
                self.project2(angle, row, column + 1, 0.0, y_intercept, PI, path)
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
        while self.player_angle >= TAU {
            self.player_angle -= TAU;
        }
        while self.player_angle < 0.0 {
            self.player_angle += TAU;
        }

        let x_component = self.player_angle.cos();
        let y_component = self.player_angle.sin();
        let mut dx = 0.0;
        let mut dy = 0.0;
        if inputs.player_forward_down {
            dx += MOVE_SPEED * x_component;
            dy += MOVE_SPEED * y_component;
        }
        if inputs.player_backward_down {
            dx -= MOVE_SPEED * x_component;
            dy -= MOVE_SPEED * y_component;
        }
        if inputs.player_strafe_left_down {
            dx += MOVE_SPEED * y_component;
            dy -= MOVE_SPEED * x_component;
        }
        if inputs.player_strafe_right_down {
            dx -= MOVE_SPEED * y_component;
            dy += MOVE_SPEED * x_component;
        }
        if self.can_move_to(self.player_x, self.player_y + dy) {
            self.player_y += dy;
        }
        if self.can_move_to(self.player_x + dx, self.player_y) {
            self.player_x += dx;
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
                // Scale for distance.
                let distance = ((self.player_x - projection.x) * (self.player_x - projection.x)
                    + (self.player_y - projection.y) * (self.player_y - projection.y))
                    .sqrt();
                // Remove fisheye effect.
                let distance = distance * (self.player_angle - angle).cos();

                // TODO: Use a numerator other than 1?
                let scale = if distance < 1.0 { 1.0 } else { 1.0 / distance };
                let height = (RENDER_HEIGHT as f32 * scale) as i32;
                let offset = (RENDER_HEIGHT as i32 - height) / 2;

                // Compute factor for diffuse lighting.
                let projection_dx = self.player_x - projection.x;
                let projection_dy = self.player_y - projection.y;
                let projection_angle = projection_dy.atan2(projection_dx);
                let angle_diff = (projection_angle - projection.normal).abs();
                let diffusion = angle_diff.cos().clamp(0.5, 1.0);

                // Compute factor for distance lighting.
                // let dimming = 1.0 + 0.00002 * distance.powf(3.5);
                let dimming = 1.0;

                let light = (diffusion / dimming).clamp(0.0, 1.0);

                let color = Color {
                    r: (projection.color.r as f32 * light) as u8,
                    g: (projection.color.g as f32 * light) as u8,
                    b: (projection.color.b as f32 * light) as u8,
                    a: projection.color.a,
                };

                context.player_batch.draw_line(
                    Point {
                        x: 320 + column,
                        y: offset,
                    },
                    Point {
                        x: 320 + column,
                        y: offset + height,
                    },
                    color,
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
