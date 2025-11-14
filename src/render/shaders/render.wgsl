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

struct Ray {
    direction: vec3f,
    origin: vec3f,
}

struct SphereIntersectResult {
    normal: vec3f,
    distance: f32,
    is_hit: bool,
}

fn sphere_intersect_check(center: vec3f, radius: f32, ray: Ray) -> SphereIntersectResult {
    var result: SphereIntersectResult;

    let delta = center - ray.origin;
    let delta_proj_len = dot(delta, ray.direction);
    let delta_proj = ray.direction * delta_proj_len;
    let h = distance(delta, delta_proj);

    result.is_hit = delta_proj_len > 0.0 && h <= radius;
    let d = sqrt(radius * radius - h * h);
    result.distance = delta_proj_len - d;
    result.normal = (delta_proj - delta - ray.direction * d) / radius;

    return result;
}

struct QuadIntersectionResult {
    distance: f32,
    is_hit: bool,
    uv: vec2f,
}

struct Quad {
    normal: vec3f,
    base: vec3f,
    u: vec3f,
    v: vec3f,
}

// Construct compact quad
fn quad_ctor(base: vec3f, u: vec3f, v: vec3f) -> Quad {
    var q: Quad;

    q.normal = normalize(cross(u, v));
    q.base = base;
    q.u = u;
    q.v = v;

    return q;
}

fn quad_intersect_check(q: Quad, ray: Ray) -> QuadIntersectionResult {
    var result: QuadIntersectionResult;

    result.distance = (dot(q.base - ray.origin, q.normal)) / dot(q.normal, ray.direction);

    let location = ray.direction * result.distance + ray.origin;

    result.uv = vec2f(dot(location - q.base, q.u), dot(location - q.base, q.v));
    result.is_hit = result.distance > 0.0
        && result.uv.x > 0.0 && result.uv.y > 0.0
        && result.uv.x < 1.0 && result.uv.y < 1.0;

    return result;
}

struct PlaneIntersectResult {
    distance: f32,
    is_hit: bool,
}

fn plane_intersect_check(point: vec3f, normal: vec3f, ray: Ray) -> PlaneIntersectResult {
    var result: PlaneIntersectResult;

    result.distance = dot(point - ray.origin, normal) / dot(normal, ray.direction);
    result.is_hit = result.distance > 0.0;
    return result;
}

struct BoxIntersectionResult {
    normal: vec3f,
    distance: f32,
    is_hit: bool,
}

fn box_intersect_check(p0: vec3f, p1: vec3f, ray: Ray) -> BoxIntersectionResult {
    let utv0 = (p0 - ray.origin) / ray.direction;
    let utv1 = (p1 - ray.origin) / ray.direction;
    let tv0 = min(utv0, utv1);
    let tv1 = max(utv0, utv1);
    let t_near = max(max(tv0.x, tv0.y), tv0.z);
    let t_far = min(min(tv1.x, tv1.y), tv1.z);
    return BoxIntersectionResult(
        /* normal:   */ vec3f(tv0 == vec3f(t_near)) * -sign(ray.direction),
        /* distance: */ select(t_far, t_near, t_near > 0.0),
        /* is_hit:   */ t_far >= max(t_near, 0.0),
    );
}

fn box_intersect_test(p0: vec3f, p1: vec3f, ray: Ray) -> bool {
    let utv0 = (p0 - ray.origin) / ray.direction;
    let utv1 = (p1 - ray.origin) / ray.direction;
    let tv0 = min(utv0, utv1);
    let tv1 = max(utv0, utv1);
    let t_near = max(max(tv0.x, tv0.y), tv0.z);
    let t_far = min(min(tv1.x, tv1.y), tv1.z);

    return t_far >= max(t_near, 0.0);
}

// Material structue
struct Material {
    color: vec3f,
    roughness: f32,
    emission: vec3f,
    metallic: f32,
}

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

//$

    return result0;
}

const MAX_BOUNCE: u32 = 12;

/// Helper for 'G' BRDF term calculation
fn ggx_partial_geometry_schlick(nd: f32, k: f32) -> f32 {
    return nd / (nd * (1 - k) + k);
}

/// 'G' Cook-Torrance BRDF term
/// Path tracing unifies the three effects essentially.
/// You simulate the direct lighting, and then there is no difference between the RT reflections and the GI.
///
/// # Note
/// `k` parameter is function from alpha and is different for IBL and direct lighting
fn ggx_geometry_smith(nv: f32, nl: f32, k: f32) -> f32 {
    return ggx_partial_geometry_schlick(nv, k) * ggx_partial_geometry_schlick(nl, k);
}

/// 'D' Cook-Torrance BRDF term
fn ggx_distribution_trowbridge_reitz(nh: f32, alpha: f32) -> f32 {
    let alpha2 = alpha * alpha;
    let den = nh * nh * (alpha2 - 1) + 1;
    return alpha2 / (radians(180) * den * den);
}

/// 'F' Cook-Torrance BRDF term
fn frensel_schlick(f0: vec3<f32>, hv: f32) -> vec3<f32> {
    return f0 + (1 - f0) * pow(1 - hv, 5);
}

/// Cook-Torrance BRDF function calculation
fn brdf_cook_torrance(
    base_color: vec3<f32>,
    metallic: f32,
    roughness: f32,
    normal: vec3<f32>,
    light: vec3<f32>,
    view: vec3<f32>,
) -> vec3<f32> {
    let nl = max(0.001, dot(normal, light));
    let nv = max(0.001, dot(normal, view));

    let half = normalize(view + light);

    let nh = max(0.001, dot(normal, half));
    let hv = max(0.001, dot(half, view));

    let alpha = roughness * roughness;

    // Alpha remapping for directional light for Smith geometry term
    let k = (alpha + 1) * (alpha + 1) / 8;

    let f0 = mix(vec3<f32>(0.04), base_color, vec3<f32>(metallic));
    let albedo = mix(base_color, vec3<f32>(0.04), vec3<f32>(metallic));

    let d = ggx_distribution_trowbridge_reitz(nh, alpha);
    let g = ggx_geometry_smith(nv, nl, k);
    let f = frensel_schlick(f0, hv);

    let diff = (1 - f) * albedo * nl / radians(180.0);
    let spec = d * g * f / (4 * nv);

    return diff + spec;
}

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
            // let sun_direction = normalize(vec3f(0.0, 2.0, -3.0));
            // let sun_radius = 0.97;
            // let sun_force = 200.0;

            // incoming_light += f32(dot(ray.direction, sun_direction) >= sun_radius) * sun_force * ray_color;
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

        // ray_color *= brdf_cook_torrance(
        //     result.color,
        //     result.metallic,
        //     result.roughness,
        //     result.normal,
        //     ray.direction,
        //     -prev_ray_direction,
        // );
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
        // out_color += trace_light / (trace_light + 0.1);
        // out_color += clamp(trace_light, vec3f(0.0), vec3f(1.0));
    }
    out_color /= f32(sample_count);

    let collected_color = textureLoad(read_collector, vec2i(frag_coord_4f.xy), 0).xyz;

    return vec4f(collected_color * f32(system.static_frame_index != 0) + out_color, 0.0);
} // fn fs_main

// file shader.wgsl
