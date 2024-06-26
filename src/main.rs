use chip_8::{Chip8, Chip8Error};
use chip_8::{HEIGHT, WIDTH};
use clap::Parser;
use env_logger::Env;
use log::{error, info};
use pixels::{Pixels, SurfaceTexture};
use std::io::Write;
use std::sync::mpsc::{channel, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};
use winit::{
    dpi::LogicalSize,
    event::Event,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit_input_helper::WinitInputHelper;

mod chip_8;

// We scale everything up by a factor of 8
const SCALE: u32 = 8;
const HZ: u32 = 30;
const CYCLES_PER_SECOND: u32 = 720;
const CYCLES_PER_FRAME: u32 = CYCLES_PER_SECOND / HZ;
const CYCLES_PER_CLOCK: u32 = CYCLES_PER_SECOND / 60;
#[derive(clap::Parser, Debug)]
struct Args {
    /// Path to the ROM that will be loaded.
    #[arg(short, long)]
    rom: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env = Env::default().default_filter_or("warn");

    env_logger::Builder::from_env(env)
        .format(|buf, record| writeln!(buf, "{}: {}", record.level(), record.args()))
        .init();

    let args = Args::parse();

    let (frame_sender, frame_receiver) = channel();
    let (input_sender, input_receiver) = channel();

    // I'm sorry I put this in a mutex, I need to multithread and the Chip8 doesn't
    // care about the performance loss.
    let mut chip_8 = Chip8::new(frame_sender, input_receiver);

    chip_8.initialize()?;

    let program_bytes = std::fs::read(args.rom)?;
    chip_8.load_program(program_bytes.clone())?;

    // Hang on to this example for dear life:
    // https://github.com/parasyte/pixels/blob/main/examples/minimal-winit/src/main.rs
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();

    let window = {
        let size = LogicalSize::new((WIDTH * SCALE) as f64, (HEIGHT * SCALE) as f64);

        WindowBuilder::new()
            .with_title("CHIP-8 Emulator")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    let mut instant = Instant::now();
    let mut last_cycle = Instant::now();
    let mut cycles = 0;
    let _game_loop = std::thread::spawn(move || loop {
        // Check for if we need to restart the program.
        if chip_8.needs_program_restart {
            chip_8.initialize().unwrap();
            info!("Restarting program...");
            chip_8.load_program(program_bytes.clone()).unwrap();
        }

        let current_cycle = Instant::now();
        if (current_cycle - last_cycle) < Duration::from_secs_f64(1f64 / (CYCLES_PER_SECOND as f64))
        {
            sleep(Duration::from_secs_f64(
                1_f64 / (2 * CYCLES_PER_SECOND) as f64,
            ));
            continue;
        }

        chip_8.cycle().unwrap();
        if Instant::now() - instant > Duration::from_secs(1) {
            info!("CPS: {}", cycles);
            cycles = 0;
            instant = Instant::now();
        }
        cycles += 1;
        last_cycle = Instant::now();
        if (cycles % 12) == 0 {
            chip_8.delay_timer.decrement();
            chip_8.sound_timer.decrement();
        }
    });
    let mut last_frame = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        // Draw the current frame
        if let Event::RedrawRequested(_) = event {
            if let Err(err) = pixels.render() {
                log_pixels_error("pixels.render", err);
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // Handle input events
        if input.update(&event) {
            // keyboard events
            let keycode_opt = crate::chip_8::keypad::handle_keyboard_input(&input, control_flow);

            dbg!(&keycode_opt);

            //dbg!(keycode_opt);
            input_sender.send(keycode_opt).unwrap();

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    log_pixels_error("pixels.resize_surface", err);
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }
            if let Ok(frame) = frame_receiver.try_recv() {
                draw_frame(&mut pixels, &frame);
            }
            if last_frame.elapsed() > Duration::from_secs_f64(1f64 / HZ as f64) {
                last_frame = Instant::now();
                window.request_redraw();
            }
        }
    });
}

fn draw_frame(winit_frame: &mut Pixels, chip_8_frame: &[u8]) {
    for (i, pixel) in winit_frame.frame_mut().chunks_exact_mut(4).enumerate() {
        let rgba = match chip_8_frame[i] {
            0 => [0, 0, 0, 0xFF],
            1 => [0xFF, 0xFF, 0xFF, 0xFF],
            _ => panic!("Invalid screen memory value."),
        };

        pixel.copy_from_slice(&rgba);
    }
}

fn log_pixels_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    if let Some(e) = err.source() {
        error!("  Caused by: {}", e);
    }
}
