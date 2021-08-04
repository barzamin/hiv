use std::time::Instant;

use anyhow::{anyhow, Result};
use shaders::ShaderConstants;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[allow(dead_code)]
struct GfxState {
    instance: wgpu::Instance,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swapchain: wgpu::SwapChain,
    size: PhysicalSize<u32>,

    render_pipeline: wgpu::RenderPipeline,

    t0: Instant,
}

impl GfxState {
    pub async fn new(window: &Window) -> Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or(anyhow!("couldn't get an adapter"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::PUSH_CONSTANTS,
                    limits: wgpu::Limits {
                        max_push_constant_size: 256,
                        ..Default::default()
                    },
                },
                None,
            )
            .await?;

        device.on_uncaptured_error(|error| {
            panic!("uncaptured wgpu error: {:#?}", error);
        });

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: adapter
                .get_swap_chain_preferred_format(&surface)
                .ok_or(anyhow!("adapter incompatible with surface format"))?,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let swapchain = device.create_swap_chain(&surface, &sc_desc);

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("shaders/lib.rs"),
            source: wgpu::util::make_spirv(include_bytes!(env!("shaders.spv"))),
            flags: wgpu::ShaderFlags::empty(), // don't validate; LLVM (probably) knows better than wgpu :3
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render pipeline layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[wgpu::PushConstantRange {
                    stages: wgpu::ShaderStage::all(),
                    range: 0..std::mem::size_of::<ShaderConstants>() as u32,
                }],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main_vs",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main_fs",
                targets: &[wgpu::ColorTargetState {
                    format: sc_desc.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0, // all samples
                alpha_to_coverage_enabled: false,
            },
        });

        let t0 = Instant::now();

        Ok(Self {
            size,
            instance,
            surface,
            adapter,
            device,
            queue,
            sc_desc,
            swapchain,
            render_pipeline,
            t0,
        })
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swapchain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn input(&mut self, evt: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
        // todo!();
    }

    fn render(&mut self, render_to: &wgpu::TextureView) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: render_to,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            let push_constants = ShaderConstants {
                width_px: self.sc_desc.width,
                height_px: self.sc_desc.height,
                time: self.t0.elapsed().as_secs_f32(),
            };

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_push_constants(
                wgpu::ShaderStage::all(),
                0,
                bytemuck::bytes_of(&push_constants),
            );
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }
}

fn main() -> Result<()> {
    let evt_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&evt_loop)?;

    let mut gfx_state = pollster::block_on(GfxState::new(&window))?;
    let mut imgui = imgui::Context::create();
    let mut im_platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    im_platform.attach_window(imgui.io_mut(), &window, imgui_winit_support::HiDpiMode::Default);
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

    let mut im_renderer = imgui_wgpu::Renderer::new(&mut imgui, &gfx_state.device, &gfx_state.queue, imgui_wgpu::RendererConfig {
        texture_format: gfx_state.sc_desc.format,
        ..Default::default()
    });

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
                        im_platform.prepare_frame(imgui.io_mut(), &window).expect("couldn't prepare imgui frame");

                        gfx_state.update();
                        gfx_state.render(&frame.output.view);

                        let ui = imgui.frame();
                        let mut op = true;
                        ui.show_demo_window(&mut op);

                        let mut encoder = gfx_state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("imgui command encoder") });
                        if last_cursor != Some(ui.mouse_cursor()) {
                            last_cursor = Some(ui.mouse_cursor());
                            im_platform.prepare_render(&ui, &window);
                        }
                        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("imgui render pass"),
                            color_attachments: &[wgpu::RenderPassColorAttachment {
                                view: &frame.output.view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load, // don't clear
                                    store: true,
                                },
                            }],
                            depth_stencil_attachment: None,
                        });
                        im_renderer.render(ui.render(), &gfx_state.queue, &gfx_state.device, &mut rpass).expect("imgui couldn't render :(");
                        drop(rpass); // TODO

                        gfx_state.queue.submit(std::iter::once(encoder.finish()));
                    },
                    Err(wgpu::SwapChainError::Lost) => gfx_state.resize(gfx_state.size), // recreate swapchain if lost
                    Err(wgpu::SwapChainError::OutOfMemory) => *ctl_flow = ControlFlow::Exit, // quit on GPU OOM
                    Err(e) => eprintln!("swap chain error: {:?}", e), // lmao don't do anything fuck swapchain errors ;3
                }
            }

            _ => (),
        }

        im_platform.handle_event(imgui.io_mut(), &window, &event);
    });
}
