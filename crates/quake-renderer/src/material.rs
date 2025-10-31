use crate::Renderer;

pub struct Material {
    bind_group: wgpu::BindGroup,
}

impl Material {
    pub const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> =
        wgpu::BindGroupLayoutDescriptor {
            label: Some("Material bind group layout"),
            entries: &[
                // Albedo texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Albedo texture sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        };

    pub fn new(
        renderer: &Renderer,
        albedo_texture: &wgpu::Texture,
        albedo_sampler: &wgpu::Sampler,
    ) -> Self {
        let albedo_texture_view =
            albedo_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let material_bind_group_layout = renderer
            .device
            .create_bind_group_layout(&Self::BIND_GROUP_LAYOUT_DESCRIPTOR);
        let material_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Material bind group"),
                layout: &material_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&albedo_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(albedo_sampler),
                    },
                ],
            });

        Self {
            bind_group: material_bind_group,
        }
    }
}
