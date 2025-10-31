use crate::Renderer;

pub struct Model {
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Model {
    pub const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> =
        wgpu::BindGroupLayoutDescriptor {
            label: Some("Model bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(
                        std::num::NonZeroU64::new(size_of::<ModelUniform>() as wgpu::BufferAddress)
                            .unwrap(),
                    ),
                },
                count: None,
            }],
        };

    pub fn new(renderer: &Renderer, transform_matrix: glam::Mat4) -> anyhow::Result<Self> {
        let model_uniform = ModelUniform::new(transform_matrix)?;

        use wgpu::util::DeviceExt;
        let model_uniform_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Model uniform buffer"),
                    contents: &model_uniform.to_bytes(),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let model_bind_group_layout = renderer
            .device
            .create_bind_group_layout(&Self::BIND_GROUP_LAYOUT_DESCRIPTOR);
        let model_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Model bind group"),
                layout: &model_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: model_uniform_buffer.as_entire_binding(),
                }],
            });

        Ok(Self {
            uniform_buffer: model_uniform_buffer,
            bind_group: model_bind_group,
        })
    }

    pub fn write_uniform_buffer(
        &self,
        renderer: &Renderer,
        transform_matrix: glam::Mat4,
    ) -> anyhow::Result<()> {
        let data = ModelUniform::new(transform_matrix)?.to_bytes();
        renderer.queue.write_buffer(&self.uniform_buffer, 0, &data);

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Default, encase::ShaderType)]
struct ModelUniform {
    transform_matrix: glam::Mat4,
    transform_inv_trans_matrix: glam::Mat3,
}

impl ModelUniform {
    fn new(transform_matrix: glam::Mat4) -> anyhow::Result<Self> {
        let upper3x3 = glam::Mat3::from_mat4(transform_matrix);
        let inverse = upper3x3.inverse();
        if !inverse.is_finite() {
            anyhow::bail!("Transform matrix is not invertible");
        }

        Ok(Self {
            transform_matrix,
            transform_inv_trans_matrix: inverse.transpose(),
        })
    }

    fn to_bytes(&self) -> Box<[u8]> {
        let mut buffer = encase::UniformBuffer::new(Vec::<u8>::new());
        buffer.write(self).unwrap();
        buffer.into_inner().into_boxed_slice()
    }
}
