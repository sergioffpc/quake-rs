use crate::Renderer;

pub struct Animation {
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Animation {
    pub const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> =
        wgpu::BindGroupLayoutDescriptor {
            label: Some("Animation bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(
                        std::num::NonZeroU64::new(
                            size_of::<AnimationUniform>() as wgpu::BufferAddress
                        )
                        .unwrap(),
                    ),
                },
                count: None,
            }],
        };

    pub fn new(renderer: &Renderer, interpolation_factor: f32) -> Self {
        let animation_uniform = AnimationUniform::new(interpolation_factor);

        use wgpu::util::DeviceExt;
        let animation_uniform_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Animation uniform buffer"),
                    contents: &animation_uniform.to_bytes(),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let animation_bind_group_layout = renderer
            .device
            .create_bind_group_layout(&Self::BIND_GROUP_LAYOUT_DESCRIPTOR);
        let animation_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Animation bind group"),
                layout: &animation_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: animation_uniform_buffer.as_entire_binding(),
                }],
            });

        Self {
            uniform_buffer: animation_uniform_buffer,
            bind_group: animation_bind_group,
        }
    }

    pub fn write_uniform_buffer(&self, renderer: &Renderer, interpolation_factor: f32) {
        let data = AnimationUniform::new(interpolation_factor).to_bytes();
        renderer.queue.write_buffer(&self.uniform_buffer, 0, &data);
    }
}

#[derive(Copy, Clone, Debug, Default, encase::ShaderType)]
struct AnimationUniform {
    interpolation_factor: f32,
}

impl AnimationUniform {
    fn new(interpolation_factor: f32) -> Self {
        Self {
            interpolation_factor,
        }
    }

    fn to_bytes(&self) -> Box<[u8]> {
        let mut buffer = encase::UniformBuffer::new(Vec::<u8>::new());
        buffer.write(self).unwrap();
        buffer.into_inner().into_boxed_slice()
    }
}
