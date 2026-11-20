//! The window, the keys and the frame loop: reads input, advances the pattern and asks the renderer to draw.

use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{debug, error, info, warn};
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::WindowBuilder,
};

use crate::render::GpuState;
use crate::state::{
    AppState, Backend, DisplayMode, COARSE_STEP, DEFAULT_COLOR_A, DEFAULT_COLOR_B,
    GRADIENT_COLOR_A, GRADIENT_COLOR_B, STEP,
};

pub(crate) fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Use all logical cores for the software path.
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_cpus_approx())
        .build_global()
        .ok();

    let event_loop = EventLoop::new().expect("event loop");
    let window = WindowBuilder::new()
        .with_title("VirtualFRC Prototypes")
        .with_inner_size(LogicalSize::new(1280, 720))
        .with_min_inner_size(LogicalSize::new(640, 360))
        .build(&event_loop)
        .expect("window");
    let window = Arc::new(window);

    let monitor_hz = window
        .current_monitor()
        .and_then(|m| m.refresh_rate_millihertz())
        .map(|mhz| mhz as f64 / 1000.0)
        .unwrap_or(60.0);
    info!("Monitor refresh rate: {:.1} Hz", monitor_hz);

    let mut state = AppState::new(monitor_hz);
    window.set_title(&state.title_string());

    let window_for_gpu = window.clone();
    let mut gpu = pollster::block_on(GpuState::new(&window_for_gpu, state.use_vsync));

    let mut title_timer = Instant::now();
    // Whether Shift is held: colors then move in bigger steps.
    let mut shift = false;

    // Frames are drawn one after the other: each ends by asking for the next one, and showing
    // a frame waits for the monitor (vsync). There is no timer and no polling, so the program
    // sleeps between refreshes and the pattern moves exactly one step per frame shown.
    window.request_redraw();

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Wait);

            match event {
                Event::WindowEvent { event, window_id } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Resized(s) => gpu.resize(s),
                        WindowEvent::ModifiersChanged(m) => shift = m.state().shift_key(),
                        WindowEvent::Moved(_) => {
                            // The window may now be on a monitor with another refresh rate.
                            let hz = window
                                .current_monitor()
                                .and_then(|m| m.refresh_rate_millihertz())
                                .map(|mhz| mhz as f64 / 1000.0);
                            if let Some(hz) = hz {
                                if (hz - state.monitor_hz).abs() > 0.5 {
                                    info!(
                                        "Monitor refresh rate: {:.1} Hz (was {:.1})",
                                        hz, state.monitor_hz
                                    );
                                    state.monitor_hz = hz;
                                    state.metrics.set_refresh_hz(hz);
                                }
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            // The gradient is always drawn by the CPU path (no shader for it).
                            let on_cpu = state.backend == Backend::Software
                                || state.mode == DisplayMode::Gradient;
                            let result = if on_cpu {
                                gpu.render_software(&state)
                            } else {
                                gpu.render_gpu(&state)
                            };
                            match result {
                                Ok(()) => {
                                    // Only a frame that was shown moves the pattern on.
                                    state.advance_phase();
                                    if let Some(report) = state.metrics.record(Instant::now()) {
                                        // Without vsync the rate is not meant to match the monitor.
                                        if state.use_vsync && report.has_problem() {
                                            warn!("Frame timing: {report}");
                                        } else {
                                            debug!("Frame timing: {report}");
                                        }
                                    }
                                    if title_timer.elapsed() >= Duration::from_millis(250) {
                                        window.set_title(&state.title_string());
                                        title_timer = Instant::now();
                                    }
                                }
                                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                                    warn!("Surface lost — reconfigure");
                                    gpu.resize(gpu.size);
                                }
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    error!("OOM");
                                    elwt.exit();
                                }
                                Err(wgpu::SurfaceError::Timeout) => {
                                    // Nothing can be shown right now (the window is hidden):
                                    // wait a little instead of asking again at full speed.
                                    std::thread::sleep(Duration::from_millis(8));
                                }
                            }
                            window.request_redraw();
                        }
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    logical_key,
                                    state: key_state,
                                    repeat: false,
                                    ..
                                },
                            ..
                        } if key_state == ElementState::Pressed => {
                            let step = if shift { COARSE_STEP } else { STEP };
                            match &logical_key {
                                Key::Named(NamedKey::Escape) => elwt.exit(),
                                Key::Named(NamedKey::Space) => state.editing_a = !state.editing_a,
                                Key::Named(NamedKey::ArrowUp) => state.adjust_channel(1, step),
                                Key::Named(NamedKey::ArrowDown) => state.adjust_channel(1, -step),
                                Key::Named(NamedKey::ArrowLeft) => state.adjust_channel(0, -step),
                                Key::Named(NamedKey::ArrowRight) => state.adjust_channel(0, step),
                                Key::Character(c) => match c.as_str() {
                                    "q" | "Q" => elwt.exit(),
                                    "m" | "M" => {
                                        state.mode = state.mode.next();
                                        info!("Mode → {}", state.mode.name());
                                    }
                                    "g" | "G" => {
                                        state.color_a = GRADIENT_COLOR_A;
                                        state.color_b = GRADIENT_COLOR_B;
                                        state.mode = DisplayMode::Gradient;
                                        info!("Mode → {}", state.mode.name());
                                    }
                                    "b" | "B" => {
                                        state.backend = state.backend.toggle();
                                        info!("Backend → {}", state.backend.name());
                                    }
                                    "1" => state.duty_a = 1,
                                    "2" => state.duty_a = 2,
                                    "3" => state.duty_a = 3,
                                    "v" | "V" => {
                                        state.use_vsync = !state.use_vsync;
                                        gpu.set_present_mode(state.use_vsync);
                                    }
                                    "[" => state.cell_size = (state.cell_size / 2).max(1),
                                    "]" => state.cell_size = (state.cell_size * 2).min(128),
                                    "r" | "R" => {
                                        state.color_a = DEFAULT_COLOR_A;
                                        state.color_b = DEFAULT_COLOR_B;
                                    }
                                    "w" | "W" => state.adjust_channel(1, step),
                                    "s" | "S" => state.adjust_channel(1, -step),
                                    "a" | "A" => state.adjust_channel(0, -step),
                                    "d" | "D" => state.adjust_channel(0, step),
                                    "e" | "E" => state.adjust_channel(2, step),
                                    "c" | "C" => state.adjust_channel(2, -step),
                                    "i" | "I" => {
                                        state.editing_a = false;
                                        state.adjust_channel(1, step);
                                    }
                                    "k" | "K" => {
                                        state.editing_a = false;
                                        state.adjust_channel(1, -step);
                                    }
                                    "j" | "J" => {
                                        state.editing_a = false;
                                        state.adjust_channel(0, -step);
                                    }
                                    "l" | "L" => {
                                        state.editing_a = false;
                                        state.adjust_channel(0, step);
                                    }
                                    "u" | "U" => {
                                        state.editing_a = false;
                                        state.adjust_channel(2, step);
                                    }
                                    _ => {}
                                },
                                _ => {}
                            }
                            window.set_title(&state.title_string());
                        }
                        _ => {}
                    }
                }

                // Some platforms ignore a redraw request made before the loop starts.
                Event::Resumed => window.request_redraw(),

                _ => {}
            }
        })
        .expect("event loop");
}

fn num_cpus_approx() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
