use crate::Renderer;

pub struct Camera {
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl Camera {
    pub const BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor<'static> =
        wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size:
                        Some(
                            std::num::NonZeroU64::new(
                                size_of::<CameraUniform>() as wgpu::BufferAddress
                            )
                            .unwrap(),
                        ),
                },
                count: None,
            }],
        };

    pub fn new(
        renderer: &Renderer,
        view_matrix: glam::Mat4,
        projection_matrix: glam::Mat4,
    ) -> Self {
        let camera_uniform = CameraUniform::new(view_matrix, projection_matrix);

        use wgpu::util::DeviceExt;
        let camera_uniform_buffer =
            renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Camera uniform buffer"),
                    contents: &camera_uniform.to_bytes(),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let camera_bind_group_layout = renderer
            .device
            .create_bind_group_layout(&Self::BIND_GROUP_LAYOUT_DESCRIPTOR);
        let camera_bind_group = renderer
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Camera bind group"),
                layout: &camera_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform_buffer.as_entire_binding(),
                }],
            });

        Self {
            uniform_buffer: camera_uniform_buffer,
            bind_group: camera_bind_group,
        }
    }

    pub fn write_uniform_buffer(
        &self,
        renderer: &Renderer,
        view_matrix: glam::Mat4,
        projection_matrix: glam::Mat4,
    ) {
        let data = CameraUniform::new(view_matrix, projection_matrix).to_bytes();
        renderer.queue.write_buffer(&self.uniform_buffer, 0, &data);
    }
}

#[derive(Copy, Clone, Debug, Default, encase::ShaderType)]
struct CameraUniform {
    view_matrix: glam::Mat4,
    projection_matrix: glam::Mat4,
    view_projection_matrix: glam::Mat4,
}

impl CameraUniform {
    fn new(view_matrix: glam::Mat4, projection_matrix: glam::Mat4) -> Self {
        Self {
            projection_matrix,
            view_matrix,
            view_projection_matrix: projection_matrix * view_matrix,
        }
    }

    fn to_bytes(&self) -> Box<[u8]> {
        let mut buffer = encase::UniformBuffer::new(Vec::<u8>::new());
        buffer.write(self).unwrap();
        buffer.into_inner().into_boxed_slice()
    }
}
