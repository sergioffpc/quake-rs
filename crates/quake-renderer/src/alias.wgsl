struct CameraUniform {
    view_matrix: mat4x4<f32>,
    projection_matrix: mat4x4<f32>,
    view_projection_matrix: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u_camera: CameraUniform;

struct ModelUniform {
    transform_matrix: mat4x4<f32>,
    transform_inv_trans_matrix: mat3x3<f32>,
};

@group(1) @binding(0)
var<uniform> u_model: ModelUniform;

struct AnimationUniform {
    interpolation_factor: f32,
};

@group(2) @binding(0)
var<uniform> u_animation: AnimationUniform;

struct VertexAttrs {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) on_seam: u32,
};

struct FragmentAttrs {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
    @location(3) on_seam: u32,
};

@group(3) @binding(0) var t_albedo: texture_2d<f32>;
@group(3) @binding(1) var s_albedo: sampler;

@vertex
fn vs_main(in: VertexAttrs) -> FragmentAttrs {
    var out: FragmentAttrs;

    let interpolated_position = mix(
        in.position,
        in.next_position,
        u_animation.interpolation_factor
    );
    let world_position = u_model.transform_matrix * vec4<f32>(interpolated_position, 1.0);

    let interpolated_normal = normalize(mix(
        in.normal,
        in.next_normal,
        u_animation.interpolation_factor
    ));
    let world_normal = normalize(u_model.transform_inv_trans_matrix * interpolated_normal);

    out.clip_position = u_camera.view_projection_matrix * world_position;
    out.world_position = world_position.xyz;
    out.world_normal = world_normal;
    out.tex_coords = in.tex_coords;
    out.on_seam = in.on_seam;

    return out;
}

@fragment
fn fs_main(in: FragmentArgs) -> @location(0) vec4<f32> {
    var tex_coords = in.tex_coords;

    // If vertex is on seam and texture coordinate is > 0.5, wrap it
    if (in.on_seam != 0u && tex_coords.x > 0.5) {
        tex_coords.x = tex_coords.x - 1.0;
    }

    let albedo_color = textureSample(t_albedo, s_albedo, tex_coords);

    return albedo_color;
}
