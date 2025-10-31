mod alias;
mod animation;
mod camera;
mod material;
mod mesh;
mod model;

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,

    camera: Option<camera::Camera>,
    materials: Vec<material::Material>,
    meshes: Vec<mesh::AliasMesh>,
}

impl Renderer {
    pub fn new<W>(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        target: W,
        width: u32,
        height: u32,
    ) -> anyhow::Result<Self>
    where
        W: raw_window_handle::HasWindowHandle
            + raw_window_handle::HasDisplayHandle
            + Send
            + Sync
            + 'static,
    {
        let surface = instance.create_surface(wgpu::SurfaceTarget::from(target))?;
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
            device,
            queue,

            camera: None,
            materials: Vec::default(),
            meshes: Vec::default(),
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
