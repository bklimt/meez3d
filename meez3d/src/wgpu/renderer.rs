use std::mem;
use std::path::Path;

use anyhow::Result;
use bytemuck::Zeroable;
use log::{error, info};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use wgpu::util::DeviceExt;
use wgpu::SurfaceTargetUnsafe;

use crate::constants::{FRAME_RATE, MAX_LIGHTS, RENDER_HEIGHT, RENDER_WIDTH};
use crate::filemanager::FileManager;
use crate::geometry::{Point, Rect};
use crate::rendercontext::{RenderContext, RenderLayer, SpriteBatch, SpriteBatchEntry};
use crate::renderer::Renderer;
use crate::sprite::Sprite;
use crate::utils::Color;
use crate::wgpu::pipeline::Pipeline;
use crate::wgpu::shader::RenderVertexUniform;
use crate::wgpu::shader::Vertex;
use crate::wgpu::shader::{self, PostprocessVertex};
use crate::wgpu::texture::Texture;

use super::shader::PostprocessFragmentUniform;

const MAX_ENTRIES: usize = 4096;
const MAX_VERTICES: usize = MAX_ENTRIES * 6;

const RECT_VERTICES: &[PostprocessVertex] = &[
    PostprocessVertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
    PostprocessVertex {
        position: [-1.0, 1.0],
        tex_coords: [0.0, 0.0],
    },
    PostprocessVertex {
        position: [-1.0, -1.0],
        tex_coords: [0.0, 1.0],
    },
    PostprocessVertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
    PostprocessVertex {
        position: [-1.0, -1.0],
        tex_coords: [0.0, 1.0],
    },
    PostprocessVertex {
        position: [1.0, -1.0],
        tex_coords: [1.0, 1.0],
    },
];

#[allow(clippy::too_many_arguments)]
fn add_rect_to_vertex_buffer(
    vertices: &mut Vec<Vertex>,
    vertex_count: &mut usize,
    destination: Rect<i32>,
    source: Rect<i32>,
    color: Color,
    reversed: bool,
    texture_atlas_width: u32,
    texture_atlas_height: u32,
) {
    let dt = destination.y as f32;
    let db = destination.bottom() as f32;
    let dl = destination.x as f32;
    let dr = destination.right() as f32;

    let st = source.y as f32;
    let sb = source.bottom() as f32;
    let mut sl = source.x as f32;
    let mut sr = source.right() as f32;

    if reversed {
        mem::swap(&mut sl, &mut sr);
    }

    // TODO: Consider moving this scaling into the shader.
    let xscale = texture_atlas_width as f32;
    let yscale = texture_atlas_height as f32;
    let st = st / yscale;
    let sb = sb / yscale;
    let sl = sl / xscale;
    let sr = sr / xscale;

    let color: [f32; 4] = color.into();

    let i = *vertex_count;
    *vertex_count += 6;

    vertices[i] = Vertex {
        position: [dl, dt],
        tex_coords: [sl, st],
        color,
    };
    vertices[i + 1] = Vertex {
        position: [dl, db],
        tex_coords: [sl, sb],
        color,
    };
    vertices[i + 2] = Vertex {
        position: [dr, dt],
        tex_coords: [sr, st],
        color,
    };
    vertices[i + 3] = Vertex {
        position: [dr, dt],
        tex_coords: [sr, st],
        color,
    };
    vertices[i + 4] = Vertex {
        position: [dl, db],
        tex_coords: [sl, sb],
        color,
    };
    vertices[i + 5] = Vertex {
        position: [dr, db],
        tex_coords: [sr, sb],
        color,
    };
}

fn add_triangle_to_vertex_buffer(
    vertices: &mut Vec<Vertex>,
    vertex_count: &mut usize,
    point1: Point<i32>,
    point2: Point<i32>,
    point3: Point<i32>,
    color: Color,
) {
    let color: [f32; 4] = color.into();

    let i = *vertex_count;
    *vertex_count += 3;

    vertices[i] = Vertex {
        position: [point1.x as f32, point1.y as f32],
        tex_coords: [0.0, 0.0],
        color,
    };
    vertices[i + 1] = Vertex {
        position: [point2.x as f32, point2.y as f32],
        tex_coords: [0.0, 0.0],
        color,
    };
    vertices[i + 2] = Vertex {
        position: [point3.x as f32, point3.y as f32],
        tex_coords: [0.0, 0.0],
        color,
    };
}

fn add_line_to_vertex_buffer(
    vertices: &mut Vec<Vertex>,
    vertex_count: &mut usize,
    point1: Point<i32>,
    point2: Point<i32>,
    color: Color,
    width: i32,
) {
    if point1.x == point2.x || point1.y == point2.y {
        panic!("a horizontal or vertical line was added to the line list");
    }

    if point1.x > point2.x {
        add_line_to_vertex_buffer(vertices, vertex_count, point2, point1, color, width);
        return;
    }

    let p1 = Point::new(point1.x as f32, point1.y as f32);
    let p2 = Point::new(point2.x as f32, point2.y as f32);
    let slope = (p2.y - p1.y) / (p2.x - p1.x);
    let perp_slope = -1.0 / slope;
    let theta = perp_slope.atan();

    let half_width = width as f32 / 2.0;
    let dx = half_width * theta.cos();
    let dy = half_width * theta.sin();
    let delta = Point::new(dx, dy);

    let delta = if perp_slope < 0.0 {
        delta * -1.0
    } else {
        delta
    };

    let q1 = p1 - delta;
    let q2 = p1 + delta;
    let q3 = p2 + delta;
    let q4 = p2 - delta;

    let color: [f32; 4] = color.into();

    let i = *vertex_count;
    *vertex_count += 6;

    vertices[i] = Vertex {
        position: [q1.x, q1.y],
        tex_coords: [0.0, 0.0],
        color,
    };
    vertices[i + 1] = Vertex {
        position: [q2.x, q2.y],
        tex_coords: [0.0, 0.0],
        color,
    };
    vertices[i + 2] = Vertex {
        position: [q3.x, q3.y],
        tex_coords: [0.0, 0.0],
        color,
    };
    vertices[i + 3] = Vertex {
        position: [q3.x, q3.y],
        tex_coords: [0.0, 0.0],
        color,
    };
    vertices[i + 4] = Vertex {
        position: [q4.x, q4.y],
        tex_coords: [0.0, 0.0],
        color,
    };
    vertices[i + 5] = Vertex {
        position: [q1.x, q1.y],
        tex_coords: [0.0, 0.0],
        color,
    };
}

pub trait WindowHandle
where
    Self: HasDisplayHandle + HasWindowHandle,
{
}

#[cfg(feature = "sdl2")]
impl WindowHandle for sdl2::video::Window {}

#[cfg(feature = "winit")]
impl WindowHandle for winit::window::Window {}

pub struct WgpuRenderer<'window, T: WindowHandle> {
    window: &'window T,
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window_width: u32,
    window_height: u32,

    render_pipeline: Pipeline,

    texture_atlas_width: u32,
    texture_atlas_height: u32,

    player_vertices: Vec<Vertex>,
    player_vertex_buffer: wgpu::Buffer,
    hud_vertices: Vec<Vertex>,
    hud_vertex_buffer: wgpu::Buffer,

    player_framebuffer: Texture,
    hud_framebuffer: Texture,
    postprocess_pipeline: Pipeline,
    postprocess_vertex_buffer: wgpu::Buffer,
    fragment_uniform: PostprocessFragmentUniform,
}

impl<'window, T> WgpuRenderer<'window, T>
where
    T: WindowHandle,
{
    // Creating some of the wgpu types requires async code
    pub async fn new(
        window: &'window T,
        window_width: u32,
        window_height: u32,
        vsync: bool,
        texture_atlas_path: &Path,
        file_manager: &FileManager,
    ) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // The surface needs to live as long as the window that created it.
        // State owns the window, so this should be safe.
        // let surface = unsafe { instance.create_surface(window).unwrap() };
        let target = unsafe { SurfaceTargetUnsafe::from_window(window)? };
        let surface = unsafe { instance.create_surface_unsafe(target)? };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let required_limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            wgpu::Limits::default()
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits,
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        info!("Reading texture atlas from {:?}", texture_atlas_path);
        let texture_atlas = Texture::from_file(&device, &queue, texture_atlas_path, file_manager)?;
        let texture_atlas_width = texture_atlas.width;
        let texture_atlas_height = texture_atlas.height;

        let surface_caps = surface.get_capabilities(&adapter);

        for format in surface_caps.formats.iter() {
            info!("available texture format: {:?}", format);
        }

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .unwrap_or(&surface_caps.formats[0]);
        info!("using texture format: {:?}", surface_format);

        let present_mode = if vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *surface_format,
            width: window_width,
            height: window_height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let mut player_vertices = Vec::new();
        player_vertices.resize_with(MAX_VERTICES, Vertex::zeroed);
        let player_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&player_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let mut hud_vertices = Vec::new();
        hud_vertices.resize_with(MAX_VERTICES, Vertex::zeroed);
        let hud_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&hud_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let postprocess_vertex_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Postprocess Vertex Buffer"),
                contents: bytemuck::cast_slice(RECT_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let mut render_pipeline = Pipeline::new(
            "Render Pipeline",
            &device,
            &shader,
            "vs_main",
            "fs_main",
            Vertex::desc(),
            &[&texture_atlas],
            config.format,
        )?;

        let vertex_uniform = RenderVertexUniform::new(RENDER_WIDTH, RENDER_HEIGHT);
        render_pipeline.set_vertex_uniform(&device, vertex_uniform);

        let player_framebuffer = Texture::frame_buffer(&device, config.format)?;
        let hud_framebuffer = Texture::frame_buffer(&device, config.format)?;
        let static_texture = Texture::static_texture(&device, &queue, RENDER_WIDTH, RENDER_HEIGHT)?;

        let mut postprocess_pipeline = Pipeline::new(
            "Postprocess Pipeline",
            &device,
            &shader,
            "vs_main2",
            "fs_main2",
            PostprocessVertex::desc(),
            &[&player_framebuffer, &hud_framebuffer, &static_texture],
            config.format,
        )?;

        let fragment_uniform = PostprocessFragmentUniform {
            texture_size: [RENDER_WIDTH as f32, RENDER_HEIGHT as f32],
            render_size: [window_width as f32, window_height as f32],
            time_s: 0.0,
            is_dark: 0,
            spotlight_count: 0,
            _padding: 0,
            spotlight: [shader::Light {
                position: [0.0, 0.0],
                radius: 0.0,
                _padding: 0.0,
            }; MAX_LIGHTS],
        };
        postprocess_pipeline.set_fragment_uniform(&device, fragment_uniform);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            window_width,
            window_height,
            render_pipeline,
            postprocess_pipeline,
            player_vertices,
            player_vertex_buffer,
            hud_vertices,
            hud_vertex_buffer,
            postprocess_vertex_buffer,
            fragment_uniform,
            texture_atlas_width,
            texture_atlas_height,
            player_framebuffer,
            hud_framebuffer,
            window,
        })
    }

    pub fn window(&self) -> &T {
        self.window
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width > 0 && new_height > 0 {
            self.window_width = new_width;
            self.window_height = new_height;
            self.config.width = new_width;
            self.config.height = new_height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn fill_vertex_buffer(&mut self, layer: RenderLayer, batch: &SpriteBatch) -> u32 {
        let (vertex_buffer, vertices) = match layer {
            RenderLayer::Player => (&self.player_vertex_buffer, &mut self.player_vertices),
            RenderLayer::Hud => (&self.hud_vertex_buffer, &mut self.hud_vertices),
        };

        if batch.entries.len() > MAX_ENTRIES {
            error!("sprite batch is too large: {}", batch.entries.len());
        }

        let mut vertex_count = 0;

        for entry in batch.entries.iter() {
            if vertex_count >= MAX_VERTICES {
                break;
            }

            match entry {
                SpriteBatchEntry::FillRect { destination, color } => {
                    let source = Rect {
                        x: 0,
                        y: 0,
                        w: 0,
                        h: 0,
                    };
                    add_rect_to_vertex_buffer(
                        vertices,
                        &mut vertex_count,
                        *destination,
                        source,
                        *color,
                        false,
                        self.texture_atlas_width,
                        self.texture_atlas_height,
                    );
                }
                SpriteBatchEntry::Sprite {
                    sprite,
                    source,
                    destination,
                    reversed,
                } => {
                    let source = Rect {
                        x: sprite.area.x + source.x,
                        y: sprite.area.y + source.y,
                        w: source.w,
                        h: source.h,
                    };
                    let color = Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 0,
                    };
                    add_rect_to_vertex_buffer(
                        vertices,
                        &mut vertex_count,
                        *destination,
                        source,
                        color,
                        *reversed,
                        self.texture_atlas_width,
                        self.texture_atlas_height,
                    );
                }
                SpriteBatchEntry::FillTriangle { p1, p2, p3, color } => {
                    add_triangle_to_vertex_buffer(
                        vertices,
                        &mut vertex_count,
                        *p1,
                        *p2,
                        *p3,
                        *color,
                    );
                }
                SpriteBatchEntry::Line {
                    start,
                    end,
                    color,
                    width,
                } => {
                    add_line_to_vertex_buffer(
                        vertices,
                        &mut vertex_count,
                        *start,
                        *end,
                        *color,
                        *width,
                    );
                }
            };
        }
        //info!("created {} vertices", vertex_count);

        self.queue.write_buffer(
            vertex_buffer,
            0,
            bytemuck::cast_slice(&vertices[0..vertex_count]),
        );

        vertex_count as u32
    }

    pub fn render(&mut self, context: &RenderContext) -> Result<()> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let vertex_count = self.fill_vertex_buffer(RenderLayer::Player, &context.player_batch);
        self.render_pipeline.render(
            &mut encoder,
            &self.player_framebuffer.view,
            context.player_batch.clear_color,
            self.player_vertex_buffer.slice(..),
            vertex_count,
        );

        let vertex_count = self.fill_vertex_buffer(RenderLayer::Hud, &context.hud_batch);
        self.render_pipeline.render(
            &mut encoder,
            &self.hud_framebuffer.view,
            context.hud_batch.clear_color,
            self.hud_vertex_buffer.slice(..),
            vertex_count,
        );

        let output = self.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let time_s = (context.frame as f32) / (FRAME_RATE as f32);
        self.fragment_uniform.time_s = time_s;

        self.fragment_uniform.is_dark = if context.is_dark { 1 } else { 0 };
        self.fragment_uniform.spotlight_count = context.lights.len() as i32;
        for (i, light) in context.lights.iter().enumerate() {
            let position = light.position;
            self.fragment_uniform.spotlight[i].position = [position.x as f32, position.y as f32];
            self.fragment_uniform.spotlight[i].radius = light.radius as f32;
        }

        self.fragment_uniform.render_size = [self.window_width as f32, self.window_height as f32];

        self.postprocess_pipeline
            .update_fragment_uniform(&self.queue, self.fragment_uniform);

        let clear_color = Color {
            r: 0,
            b: 0,
            g: 0,
            a: 255,
        };
        self.postprocess_pipeline.render(
            &mut encoder,
            &output_view,
            clear_color,
            self.postprocess_vertex_buffer.slice(..),
            6,
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        output.present();

        Ok(())
    }
}

impl<'window, T> Renderer for WgpuRenderer<'window, T>
where
    T: WindowHandle,
{
    fn load_sprite(&mut self, _path: &Path) -> Result<Sprite> {
        // TODO: Check that the path actually matches the texture_atlas_path.
        Ok(Sprite {
            id: 0,
            area: Rect {
                x: 0,
                y: 0,
                w: self.texture_atlas_width as i32,
                h: self.texture_atlas_height as i32,
            },
        })
    }
}
