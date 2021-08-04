use std::time::Instant;

use anyhow::{anyhow, Result};
use shaders::ShaderConstants;
use winit::{dpi::PhysicalSize, event::WindowEvent, window::Window};

#[allow(dead_code)]
pub struct GfxState {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub sc_desc: wgpu::SwapChainDescriptor,
    pub swapchain: wgpu::SwapChain,
    pub size: PhysicalSize<u32>,

    pub render_pipeline: wgpu::RenderPipeline,

    pub t0: Instant,
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

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swapchain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn input(&mut self, evt: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {
        // todo!();
    }

    pub fn render(&mut self, render_to: &wgpu::TextureView) {
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

pub struct ImguiPipeline {
    pub platform: imgui_winit_support::WinitPlatform,
    pub renderer: imgui_wgpu::Renderer,
}

impl ImguiPipeline {
    pub fn new(imgui: &mut imgui::Context, window: &Window, device: &wgpu::Device, queue: &wgpu::Queue, texture_format: wgpu::TextureFormat) -> Self {
        let mut platform = imgui_winit_support::WinitPlatform::init(imgui);
        platform.attach_window(imgui.io_mut(), &window, imgui_winit_support::HiDpiMode::Default);

        let renderer = imgui_wgpu::Renderer::new(imgui, device, queue, imgui_wgpu::RendererConfig {
            texture_format,
            ..Default::default()
        });

        Self {
            platform,
            renderer,
        }
    }

    pub fn render(&mut self, draw_data: &imgui::DrawData, device: &wgpu::Device, queue: &wgpu::Queue, target_tex: &wgpu::TextureView) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("imgui command encoder") });
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("imgui render pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: target_tex,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // don't clear
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });
        self.renderer.render(draw_data, queue, device, &mut rpass).expect("imgui couldn't render :(");
        drop(rpass); // TODO

        queue.submit(std::iter::once(encoder.finish()));
    }
}
