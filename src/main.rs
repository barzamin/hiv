//! doggy park
//!
//! # Notes:
//! - when dpi factor changes (move window), we should fixup imgui's scaling
use std::{fs, path::Path, sync::{Arc, Mutex, mpsc::channel}, thread, time::{Duration, Instant}};

use anyhow::Result;
use imgui::im_str;
use notify::{RecursiveMode, Watcher};
use spirv_builder::{MetadataPrintout, SpirvBuilder};
use tracing::{Level, error, info, span};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod render;
use render::{GfxState, ImguiPipeline};


fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let evt_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&evt_loop)?;

    // -- shader auto-reload watch & build thread
    // potential improvements:
    //  - instead of a Mutex<Option<T>>, consider using some sort of atomically-swappable container
    let debounce = Duration::from_secs(5);
    let (fs_tx, fs_rx) = channel();
    let new_spirv: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
    let mut watcher = notify::recommended_watcher(move |res| {
        match res {
            Ok(evt) => { fs_tx.send(evt).unwrap(); },
            Err(err) => eprintln!("[shaders fswatch] error: {:?}", err),
        }
    })?;
    watcher.watch(Path::new("./shaders"), RecursiveMode::Recursive)?;
    let send_spirv = Arc::clone(&new_spirv);
    thread::spawn(move || {
        loop {
            let _ = fs_rx.recv().unwrap(); // wait on one event
            // debounce: wait for any filesystem activity to calm down
            while let Ok(_) = fs_rx.recv_timeout(debounce) {
            }

            let sp = span!(Level::INFO, "shader build");
            let _ent = sp.enter();

            info!("rebuilding");
            match SpirvBuilder::new("./shaders", "spirv-unknown-vulkan1.1")
                .print_metadata(MetadataPrintout::None)
                .build()
            {
                Ok(built) => {
                    info!(result = ?built, "successfully built shaders");
                    let path = built.module.unwrap_single();
                    let spirv = fs::read(path).expect("build successful; artifact should exist");

                    let _ = send_spirv.lock().unwrap().replace(spirv);
                },
                Err(err) => error!(error = ?err, "error rebuilding"),
            }

            drop(_ent);
        }
    });

    let mut gfx_state = pollster::block_on(GfxState::new(&window))?;
    let mut imgui = imgui::Context::create();
    let mut im_pipe = ImguiPipeline::new(&mut imgui, &window, &gfx_state.device, &gfx_state.queue, gfx_state.sc_desc.format);
    imgui.set_ini_filename(None);

    let hidpi_factor = im_pipe.platform.hidpi_factor();
    let font_size = (13.0 * hidpi_factor) as f32;
    imgui.fonts().add_font(&[imgui::FontSource::DefaultFontData {
        config: Some(imgui::FontConfig {
            // oversample_h: 1,
            // pixel_snap_h: true,
            size_pixels: font_size,
            ..Default::default()
        }),
    }]);
    imgui.io_mut().font_global_scale = (1.0/hidpi_factor) as f32;


    let mut last_frame = Instant::now();
    let mut render_step_cnt = false;
    evt_loop.run(move |event, _, ctl_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                if !gfx_state.input(event) {
                    match event {
                        WindowEvent::CloseRequested => *ctl_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physize) => {
                            gfx_state.resize(*physize);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            gfx_state.resize(**new_inner_size);
                        }
                        _ => (),
                    }
                }
            }

            Event::MainEventsCleared => {
                im_pipe.platform.prepare_frame(imgui.io_mut(), &window).expect("couldn't prepare imgui frame");
                window.request_redraw(); // always request a new frame
            }

            Event::RedrawRequested(_) => {
                let now = Instant::now();
                let delta_s = now - last_frame;
                imgui.io_mut().update_delta_time(delta_s);
                last_frame = now;

                match gfx_state.swapchain.get_current_frame() {
                    Ok(frame) => {

                        if let Some(spirv) = new_spirv.lock().unwrap().take() {
                            gfx_state.load_shader_code(&spirv);
                        }

                        gfx_state.update();
                        gfx_state.render(&frame.output.view);

                        let ui = imgui.frame();
                        imgui::Window::new(im_str!("trace control"))
                            .size([300., 100.], imgui::Condition::FirstUseEver)
                            .build(&ui, || {
                            });

                        im_pipe.platform.prepare_render(&ui, &window);

                        im_pipe.render(ui.render(), &gfx_state.device, &gfx_state.queue, &frame.output.view);
                    },
                    Err(wgpu::SwapChainError::Lost) => gfx_state.resize(gfx_state.size), // recreate swapchain if lost
                    Err(wgpu::SwapChainError::OutOfMemory) => *ctl_flow = ControlFlow::Exit, // quit on GPU OOM
                    Err(e) => eprintln!("swap chain error: {:?}", e), // lmao don't do anything fuck swapchain errors ;3
                }
            }

            _ => (),
        }

        im_pipe.platform.handle_event(imgui.io_mut(), &window, &event);
    });
}
