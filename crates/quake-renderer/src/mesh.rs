use crate::Renderer;

pub struct AliasMesh {
    positions_buffer: wgpu::Buffer,
    next_positions_buffer: wgpu::Buffer,
    normals_buffer: wgpu::Buffer,
    next_normals_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl AliasMesh {
    pub fn from_mdl(renderer: &Renderer, mdl: &quake_model::mdl::Mdl) -> Self {
        let vertex_buffer_size = (mdl.vertices_count as usize * size_of::<glam::Vec3>()) as u64;

        let positions_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Alias mesh positions buffer"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let next_positions_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Alias mesh next positions buffer"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let normals_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Alias mesh normals buffer"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let next_normals_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Alias mesh next normals buffer"),
            size: vertex_buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let indices = mdl
            .triangles
            .as_ref()
            .into_iter()
            .flat_map(|t| [t.indices.x, t.indices.y, t.indices.z])
            .collect::<Vec<_>>();

        use wgpu::util::DeviceExt;
        let index_buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Alias mesh index buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Self {
            positions_buffer,
            next_positions_buffer,
            normals_buffer,
            next_normals_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }

    pub fn write_vertex_buffer(
        &self,
        renderer: &Renderer,
        positions: Box<[glam::Vec3]>,
        next_positions: Box<[glam::Vec3]>,
        normals: Box<[glam::Vec3]>,
        next_normals: Box<[glam::Vec3]>,
    ) {
        renderer
            .queue
            .write_buffer(&self.positions_buffer, 0, bytemuck::cast_slice(&positions));
        renderer.queue.write_buffer(
            &self.next_positions_buffer,
            0,
            bytemuck::cast_slice(&next_positions),
        );
        renderer
            .queue
            .write_buffer(&self.normals_buffer, 0, bytemuck::cast_slice(&normals));
        renderer.queue.write_buffer(
            &self.next_normals_buffer,
            0,
            bytemuck::cast_slice(&next_normals),
        );
    }
}
