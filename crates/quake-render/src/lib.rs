pub struct RenderManager {
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

impl RenderManager {
    pub async fn new<W>(target: W, width: u32, height: u32) -> anyhow::Result<Self>
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
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Renderer device"),
                memory_hints: wgpu::MemoryHints::Performance,
                ..Default::default()
            })
            .await?;
        let surface_config = surface.get_default_config(&adapter, width, height).unwrap();
        surface.configure(&device, &surface_config);

        Ok(Self {
            surface,
            surface_config,
            device,
            queue,
        })
    }
}
