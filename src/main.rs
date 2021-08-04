use std::time::Instant;

use anyhow::Result;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod render;
use render::{GfxState, ImguiPipeline};

fn main() -> Result<()> {
    let evt_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&evt_loop)?;

    let mut gfx_state = pollster::block_on(GfxState::new(&window))?;
    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);

    let font_size = (13.0 * window.scale_factor()) as f32;
    imgui.io_mut().font_global_scale = (1.0/window.scale_factor()) as f32;
    imgui.fonts().add_font(&[imgui::FontSource::DefaultFontData {
        config: Some(imgui::FontConfig {
            oversample_h: 1,
            pixel_snap_h: true,
            size_pixels: font_size,
            ..Default::default()
        }),
    }]);
    let mut im_pipe = ImguiPipeline::new(&mut imgui, &window, &gfx_state.device, &gfx_state.queue, gfx_state.sc_desc.format);

    let mut last_frame = Instant::now();
    let mut last_cursor = None;
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
                window.request_redraw(); // always request a new frame
            }

            Event::RedrawRequested(_) => {
                let now = Instant::now();
                let delta_s = now - last_frame;
                imgui.io_mut().update_delta_time(delta_s);
                last_frame = now;

                match gfx_state.swapchain.get_current_frame() {
                    Ok(frame) => {
                        im_pipe.platform.prepare_frame(imgui.io_mut(), &window).expect("couldn't prepare imgui frame");

                        gfx_state.update();
                        gfx_state.render(&frame.output.view);

                        let ui = imgui.frame();
                        let mut op = true;
                        ui.show_demo_window(&mut op);

                        if last_cursor != Some(ui.mouse_cursor()) {
                            last_cursor = Some(ui.mouse_cursor());
                            im_pipe.platform.prepare_render(&ui, &window);
                        }

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
