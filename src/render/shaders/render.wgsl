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

struct Camera {
    location: vec3f,
    direction: vec3f,
    near: f32,
    right: vec3f,
    projection_width: f32,
    up: vec3f,
    projection_height: f32,
}

struct System {
    resolution: vec2<f32>,
    time: f32,
    static_frame_index: u32,
    resolution_scale: vec2<f32>,
    texel_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var<uniform> system: System;
@group(1) @binding(0) var read_collector: texture_2d<f32>;

var<private> _rand_seed : u32 = 123456789;

fn rand_u32() -> u32 {
    _rand_seed ^= _rand_seed << 13;
    _rand_seed ^= _rand_seed >> 17;
    _rand_seed ^= _rand_seed << 5;
    return _rand_seed;
}

fn rand_f32() -> f32 {
    return f32(rand_u32()) / 4294967295.0;
}

fn rand_vec3() -> vec3f {
    let theta = radians(360.0) * rand_f32();
    let phi = acos(1.0 - 2.0 * rand_f32());
    return vec3f(
        sin(phi) * cos(theta),
        cos(phi),
        sin(phi) * sin(theta),
    );
}

/// Ray structure
struct Ray {
    direction: vec3f,
    origin: vec3f,
}

/// Material structue
struct Material {
    color: vec3f,
    roughness: f32,
    emission: vec3f,
    metallic: f32,
}

/// Result of intersection between
struct IntersectionResult {
    distance: f32,
    is_hit: bool,
    normal: vec3f,
    material: Material,
}

// Scene intersection result
fn intersect_scene(ray: Ray) -> IntersectionResult {
    var result0: IntersectionResult;

    result0.is_hit = false;
    result0.distance = 8000000000.0;

    //$SCENE Cpu-generated code goes here

    return result0;
}

const MAX_BOUNCE: u32 = 16;

/// Lambertian BRDF
fn brdf_lambert(
    base_color: vec3<f32>,
    normal: vec3<f32>,
    light_direction: vec3<f32>
) -> vec3<f32> {
    return base_color * clamp(dot(normal, light_direction), 0.0, 1.0) * (1.0 / radians(180.0));
}

fn trace(init_ray: Ray) -> vec3f {
    var ray_color = vec3f(1.0, 1.0, 1.0);
    var incoming_light = vec3f(0.0, 0.0, 0.0);
    var ray = init_ray;

    var index = MAX_BOUNCE + 1;

    while true {
        // let result = intersect_cornell(ray);
        let result = intersect_scene(ray);

        // Calculate sun emission on scene intersection fail
        if !result.is_hit {
            break;
        }

        incoming_light += result.material.emission * ray_color.rgb;

        index -= 1;
        if index == 0 {
            break;
        }

        let prev_ray_direction = ray.direction;

        ray.origin += ray.direction * result.distance + result.normal * 0.001;

        // Trace hemisphere-evenly-distributed random ray
        ray.direction = rand_vec3();
        ray.direction *= sign(dot(ray.direction, result.normal));

        ray_color *= brdf_lambert(
            result.material.color,
            result.normal,
            ray.direction,
        );
    }

    return incoming_light;
}

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

fn tex_coord_to_ray(tex_coord: vec2f) -> Ray {
    let coord = tex_coord * 2.0 - 1.0;
    var ray: Ray;
    ray.origin = camera.location;
    ray.direction = normalize(camera.direction * camera.near
        + camera.right * camera.projection_width * coord.x
        + camera.up * camera.projection_height * coord.y
    );

    return ray;
}

@fragment
fn fs_main(@builtin(position) frag_coord_4f: vec4f, @location(0) tex_coord: vec2f) -> @location(0) vec4f {
    _rand_seed = 1
        * u32(tex_coord.x * 3123456.0)
        * u32(tex_coord.y * 8765345.0)
        * u32(fract(system.time) * 324234234.5);

    let sample_count = 4;

    var out_color = vec3f(0.0, 0.0, 0.0);
    for (var i = 0; i < sample_count; i++) {
        let trace_dir = tex_coord_to_ray(tex_coord + system.texel_size * vec2f(rand_f32(), rand_f32()));
        let trace_light = max(trace(trace_dir), vec3f(0.0));

        out_color += tonemap_aces_approx(trace_light);
    }
    out_color /= f32(sample_count);

    let collected_color = textureLoad(read_collector, vec2i(frag_coord_4f.xy), 0).xyz;

    return vec4f(collected_color * f32(system.static_frame_index != 0) + out_color, 0.0);
} // fn fs_main

// file shader.wgsl
