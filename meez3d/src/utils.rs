use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{anyhow, bail, Error, Result};

use crate::geometry::Rect;

/*
#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn opposite(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Right => Direction::Left,
            Direction::Left => Direction::Right,
        }
    }
}

impl FromStr for Direction {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "N" => Direction::Up,
            "S" => Direction::Down,
            "W" => Direction::Left,
            "E" => Direction::Right,
            _ => bail!("invalid direction: {}", s),
        })
    }
}
*/

#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl FromStr for Color {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if s.len() == 6 {
            let r = u8::from_str_radix(&s[0..2], 16)?;
            let g = u8::from_str_radix(&s[2..4], 16)?;
            let b = u8::from_str_radix(&s[4..6], 16)?;
            Ok(Color { r, g, b, a: 255 })
        } else if s.len() == 8 {
            let a = u8::from_str_radix(&s[0..2], 16)?;
            let r = u8::from_str_radix(&s[2..4], 16)?;
            let g = u8::from_str_radix(&s[4..6], 16)?;
            let b = u8::from_str_radix(&s[6..8], 16)?;
            Ok(Color { r, g, b, a })
        } else {
            Err(anyhow!("invalid color: {}", s))
        }
    }
}

#[cfg(feature = "wgpu")]
impl From<Color> for wgpu::Color {
    fn from(value: Color) -> Self {
        wgpu::Color {
            r: value.r as f64 / 255.0,
            g: value.g as f64 / 255.0,
            b: value.b as f64 / 255.0,
            a: value.a as f64 / 255.0,
        }
    }
}

impl From<Color> for [f32; 4] {
    fn from(value: Color) -> Self {
        [
            value.r as f32 / 255.0,
            value.g as f32 / 255.0,
            value.b as f32 / 255.0,
            value.a as f32 / 255.0,
        ]
    }
}

pub fn normalize_path(path: &Path) -> Result<PathBuf> {
    let mut output = PathBuf::new();
    for part in path.iter() {
        if part == ".." {
            if !output.pop() {
                output.push(part);
            }
        } else {
            output.push(part);
        }
    }
    Ok(output)
}
