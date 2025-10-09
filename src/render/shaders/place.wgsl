/// P_TR Project
/// `File` render/shader.wgsl
/// `Description` Path tracig module.
/// `Author` TioT2
/// `Last changed` 18.02.2024

struct VsOut {
    @builtin(position) ndc_position: vec4f,
    @location(0) tex_coord: vec2f,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    var tex_coord = vec2f(
        f32(index / 2),
        f32(index % 2)
    );

    return VsOut(
        vec4f(tex_coord * 2.0 - 1.0, 0.0, 1.0),
        tex_coord
    );
}

struct System {
    resolution: vec2<f32>,
    time: f32,
    static_frame_index: u32,
    resolution_scale: vec2<f32>,
    texel_size: vec2<f32>,
}

@group(0) @binding(1) var<uniform> system: System;
@group(1) @binding(0) var light_collector: texture_2d<f32>;

@fragment
fn fs_main(@builtin(position) frag_coord_4f: vec4f, @location(0) tex_coord: vec2f) -> @location(0) vec4f {
    let light = textureLoad(light_collector, vec2i(frag_coord_4f.xy / system.resolution_scale), 0) / f32(system.static_frame_index + 1);

    // return light / (light + 1.0);
    return light;
} // fn fs_main

// file shader.wgsl
