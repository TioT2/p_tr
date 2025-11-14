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

// ACES tonemapping operator performant approximation
fn tonemap_aces_approx(light: vec3f) -> vec3f {
    const a = 2.51;
    const b = 0.03;
    const c = 2.43;
    const d = 0.59;
    const e = 0.14;
    let v = light * 0.6;
    return clamp((v * (a * v + b)) / (v * (c * v + d) + e), vec3f(0.0), vec3f(1.0));
}

@fragment
fn fs_main(@builtin(position) frag_coord_4f: vec4f, @location(0) tex_coord: vec2f) -> @location(0) vec4f {
    let collector_coord = vec2i(frag_coord_4f.xy / system.resolution_scale);
    let collected_color = textureLoad(light_collector, collector_coord, 0).xyz;
    let compressed_color = tonemap_aces_approx(collected_color);

    return vec4f(compressed_color, 0.0);
} // fn fs_main

// file shader.wgsl
