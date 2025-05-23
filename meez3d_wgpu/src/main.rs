use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use clap::Parser;
use sdl2::event::{Event, WindowEvent};

use meez3d::{
    FileManager, ImageManager, InputManager, RecordOption, RenderContext, SoundManager,
    StageManager, WgpuRenderer, FRAME_RATE, RENDER_HEIGHT, RENDER_WIDTH,
};

pub const WINDOW_WIDTH: u32 = 1600;
pub const WINDOW_HEIGHT: u32 = 900;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    pub fullscreen: bool,

    #[arg(long)]
    pub assets: Option<String>,
}

fn run(args: Args) -> Result<()> {
    let sdl_context = sdl2::init().expect("failed to init SDL");
    let video_subsystem = sdl_context.video().expect("failed to get video context");
    let audio_subsystem = sdl_context.audio().expect("failed to get audio context");

    let file_manager = match &args.assets {
        Some(path) => FileManager::from_archive_file(Path::new(path)),
        None => FileManager::from_fs(),
    }?;

    let title = "flywheel";
    let mut window = video_subsystem.window(title, WINDOW_WIDTH, WINDOW_HEIGHT);
    if args.fullscreen {
        window.fullscreen_desktop();
    }
    let window = window.resizable().build().expect("failed to build window");
    let (width, height) = window.size();
    sdl_context.mouse().show_cursor(false);

    let texture_atlas_path = Path::new("assets/textures.png");
    let future = WgpuRenderer::new(
        &window,
        width,
        height,
        false,
        texture_atlas_path,
        &file_manager,
    );
    let renderer = pollster::block_on(future)?;

    let mut image_manager: ImageManager<WgpuRenderer<'_, sdl2::video::Window>> =
        ImageManager::new(renderer)?;
    image_manager.load_texture_atlas(
        Path::new("assets/textures.png"),
        Path::new("assets/textures_index.txt"),
        &file_manager,
    )?;
    let font = image_manager.load_font(&file_manager)?;

    let mut input_manager = InputManager::with_options(
        WINDOW_WIDTH as i32,
        WINDOW_HEIGHT as i32,
        true,
        RecordOption::None,
        &file_manager,
    )?;

    let mut stage_manager = StageManager::new(&file_manager, &mut image_manager)?;
    let mut sound_manager = SoundManager::with_sdl(&audio_subsystem)?;
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut frame = 0;
    let speed_test_start_time: Instant = Instant::now();

    'running: loop {
        let start_time = Instant::now();

        let width = RENDER_WIDTH;
        let height = RENDER_HEIGHT;
        let mut context = RenderContext::new(width, height, frame)?;

        for event in event_pump.poll_iter() {
            input_manager.handle_sdl_event(&event);
            match event {
                Event::Quit { .. } => break 'running,
                Event::Window {
                    win_event: WindowEvent::SizeChanged(new_width, new_height),
                    window_id,
                    ..
                } if window_id == window.id() => {
                    image_manager
                        .renderer_mut()
                        .resize(new_width as u32, new_height as u32);
                }
                _ => {}
            }
        }

        let input_snapshot = input_manager.update(frame);

        if !stage_manager.update(
            &context,
            &input_snapshot,
            &file_manager,
            &mut image_manager,
            &mut sound_manager,
        )? {
            break 'running;
        }

        context.clear();
        stage_manager.draw(&mut context, &font);
        image_manager
            .renderer_mut()
            .render(&context)
            .map_err(|e| anyhow!("rendering error: {}", e))?;

        frame += 1;
        let target_duration = Duration::new(0, 1_000_000_000u32 / FRAME_RATE);
        let actual_duration = start_time.elapsed();
        if actual_duration > target_duration {
            continue;
        }
        let remaining = target_duration - actual_duration;
        ::std::thread::sleep(remaining);
    }

    let speed_test_end_time = Instant::now();
    let speed_test_duration = speed_test_end_time - speed_test_start_time;
    let fps = frame as f64 / speed_test_duration.as_secs_f64();

    Ok(())
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    match run(args) {
        Ok(_) => {}
        Err(e) => panic!("{}", e),
    }
}
