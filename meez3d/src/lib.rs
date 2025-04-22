#![allow(clippy::manual_range_contains, clippy::collapsible_else_if)]

mod constants;
mod cursor;
mod filemanager;
mod font;
mod geometry;
mod imagemanager;
mod inputmanager;
mod level;
mod menu;
mod properties;
mod rendercontext;
mod renderer;
mod scene;
mod smallintmap;
mod smallintset;
mod soundmanager;
mod sprite;
mod stagemanager;
mod tilemap;
mod tileset;
mod uibutton;
mod utils;

pub use constants::{FRAME_RATE, RENDER_HEIGHT, RENDER_WIDTH};

pub use filemanager::FileManager;
pub use font::Font;
pub use imagemanager::ImageManager;
pub use inputmanager::{InputManager, RecordOption};
pub use rendercontext::RenderContext;
pub use soundmanager::{Sound, SoundManager, SoundPlayer};
pub use stagemanager::StageManager;

#[cfg(feature = "sdl2")]
mod sdl;

#[cfg(feature = "wgpu")]
mod wgpu;

#[cfg(feature = "wgpu")]
pub use wgpu::renderer::WgpuRenderer;
