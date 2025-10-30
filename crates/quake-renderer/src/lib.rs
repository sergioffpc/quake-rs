pub struct Renderer {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl Renderer {
    pub fn new<W>(target: W, width: u32, height: u32) -> anyhow::Result<Self>
    where
        W: raw_window_handle::HasWindowHandle
            + raw_window_handle::HasDisplayHandle
            + Send
            + Sync
            + 'static,
    {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            flags: wgpu::InstanceFlags::DEBUG | wgpu::InstanceFlags::VALIDATION,
            ..Default::default()
        });
        let surface = instance.create_surface(wgpu::SurfaceTarget::from(target))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("Renderer device"),
                ..Default::default()
            }))?;
        let surface_config = surface.get_default_config(&adapter, width, height).unwrap();
        surface.configure(&device, &surface_config);

        Ok(Self {
            surface,
            surface_config,
            adapter,
            device,
            queue,
        })
    }

    pub fn present(&self) -> anyhow::Result<()> {
        let present_texture = self.surface.get_current_texture()?;
        let present_view = present_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                label: Some("Present texture view"),
                format: Some(self.surface_config.format),
                ..Default::default()
            });

        let mut command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Present command encoder"),
                });

        {
            let render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Present render pass"),
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
            });
        }

        self.queue.submit(std::iter::once(command_encoder.finish()));
        present_texture.present();

        Ok(())
    }
}
