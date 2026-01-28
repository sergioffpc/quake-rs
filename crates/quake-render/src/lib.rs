pub struct RenderManagerDescriptor<'a> {
    pub display_handle: &'a dyn wgpu::rwh::HasDisplayHandle,
    pub window_handle: &'a dyn wgpu::rwh::HasWindowHandle,
    pub width: u32,
    pub height: u32,
}

pub struct RenderManager {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,

    surface_texture: Option<wgpu::SurfaceTexture>,
    command_encoder: Option<wgpu::CommandEncoder>,
}

impl RenderManager {
    pub fn new(desc: &RenderManagerDescriptor<'_>) -> anyhow::Result<Self> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::default(),
            flags: wgpu::InstanceFlags::debugging(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
        });

        let window_handle = desc.window_handle.window_handle()?;
        let display_handle = desc.display_handle.display_handle()?;

        let surface = unsafe {
            let target = wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: display_handle.as_raw(),
                raw_window_handle: window_handle.as_raw(),
            };
            instance.create_surface_unsafe(target)
        }?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))?;

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::default(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::default(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            }))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_caps.formats[0],
            width: desc.width,
            height: desc.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 0,
            alpha_mode: wgpu::CompositeAlphaMode::default(),
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        Ok(Self {
            surface,
            surface_config,
            device,
            queue,
            surface_texture: None,
            command_encoder: None,
        })
    }

    pub fn destroy(&mut self) {
        self.surface_texture = None;
        self.command_encoder = None;
    }

    pub fn on_resize(&mut self, width: u32, height: u32) {
        self.surface_texture = None;
        self.command_encoder = None;

        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn on_acquire_frame(&mut self) -> anyhow::Result<()> {
        let surface_texture = self.surface.get_current_texture()?;
        self.surface_texture = Some(surface_texture);

        let command_encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.command_encoder = Some(command_encoder);

        Ok(())
    }

    pub fn on_draw_frame(&mut self) {
        let present_texture = self.surface_texture.as_ref().unwrap();
        let present_view = present_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut command_encoder = self.command_encoder.take().unwrap();
        {
            let _render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &present_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        self.queue.submit(std::iter::once(command_encoder.finish()));
    }

    pub fn on_present_frame(&mut self) {
        if let Some(surface_texture) = self.surface_texture.take() {
            surface_texture.present();
        }
    }
}
