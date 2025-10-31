use crate::Renderer;
use crate::animation::Animation;
use crate::camera::Camera;
use crate::material::Material;
use crate::model::Model;

pub struct AliasRenderObject {
    transform_matrix: glam::Mat4,
    interpolation_factor: f32,

    material_index: u32,
}

pub struct AliasRenderPipeline {
    render_pipeline: wgpu::RenderPipeline,
}

impl AliasRenderPipeline {
    pub fn new(renderer: &Renderer) -> Self {
        let shader_module = renderer
            .device
            .create_shader_module(wgpu::include_wgsl!("alias.wgsl"));

        let bind_group_layouts = [
            Camera::BIND_GROUP_LAYOUT_DESCRIPTOR,
            Model::BIND_GROUP_LAYOUT_DESCRIPTOR,
            Animation::BIND_GROUP_LAYOUT_DESCRIPTOR,
            Material::BIND_GROUP_LAYOUT_DESCRIPTOR,
        ]
        .map(|it| renderer.device.create_bind_group_layout(&it));

        let render_pipeline_layout =
            renderer
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Alias render pipeline layout"),
                    bind_group_layouts: &bind_group_layouts
                        .iter()
                        .collect::<Vec<&wgpu::BindGroupLayout>>(),
                    push_constant_ranges: &[],
                });
        let render_pipeline =
            renderer
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Alias render pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader_module,
                        entry_point: Some("vs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[
                            // Positions buffer
                            wgpu::VertexBufferLayout {
                                array_stride: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                                step_mode: wgpu::VertexStepMode::Vertex,
                                attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                            },
                            // Normals buffer
                            wgpu::VertexBufferLayout {
                                array_stride: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                                step_mode: wgpu::VertexStepMode::Vertex,
                                attributes: &wgpu::vertex_attr_array![2 => Float32x3],
                            },
                            // Texture coordinates and flags buffer
                            wgpu::VertexBufferLayout {
                                array_stride: size_of::<[f32; 2]>() as wgpu::BufferAddress,
                                step_mode: wgpu::VertexStepMode::Vertex,
                                attributes: &wgpu::vertex_attr_array![4 => Float32x2],
                            },
                            wgpu::VertexBufferLayout {
                                array_stride: size_of::<u32>() as wgpu::BufferAddress,
                                step_mode: wgpu::VertexStepMode::Vertex,
                                attributes: &wgpu::vertex_attr_array![5 => Uint32],
                            },
                        ],
                    },
                    primitive: wgpu::PrimitiveState {
                        cull_mode: Some(wgpu::Face::Back),
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &shader_module,
                        entry_point: Some("fs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: renderer.surface_config.format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: Default::default(),
                        })],
                    }),
                    multiview: None,
                    cache: None,
                });

        Self { render_pipeline }
    }
}
