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
    resolution: vec2f,
    time: f32,
    static_frame_index: u32,
    texel_size: vec2f,
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
    let theta = 2 * 3.14159265367989 * rand_f32();
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
        /* distance: */ mix(t_far, t_near, f32(t_near > 0.0)),
        /* is_hit:   */ t_far >= max(t_near, 0.0),
    );
}

fn box_intersect_test(p0: vec3f, p1: vec3f, ray: Ray) -> bool {
    let utv0 = (p0 - ray.origin) / ray.direction;
    let utv1 = (p1 - ray.origin) / ray.direction;
    let tv0 = min(utv0, utv1);
    let tv1 = max(utv0, utv1);
    return min(min(tv1.x, tv1.y), tv1.z) >= max(max(max(tv0.x, tv0.y), tv0.z), 0.0);
}

struct SceneIntersectionResult {
    color: vec3f,
    distance: f32,
    emission: vec3f,
    is_hit: bool,
    normal: vec3f,
}

fn intersect_scene(ray: Ray) -> SceneIntersectionResult {
    var result: SceneIntersectionResult;

    result.is_hit = false;
    result.distance = 100000000.0;

    {
        let i = sphere_intersect_check(vec3f(0.0, 2.0, -3.0), 1.0, ray);

        if i.is_hit && i.distance < result.distance {
            result.is_hit = true;
            result.distance = i.distance;
            result.color = vec3f(1.0, 1.0, 1.0);
            result.emission = vec3f(1.0, 1.0, 1.0);
            result.normal = i.normal;
            // result.roughness = 1.0;
        }
    }

    {
        let i = sphere_intersect_check(vec3f(1.1, 0.55, -1.1), 0.5, ray);

        if i.is_hit && i.distance < result.distance {
            result.is_hit = true;
            result.distance = i.distance;
            result.color = vec3f(0.30, 0.47, 0.80);
            result.emission = vec3f(0.0, 0.0, 0.0);
            result.normal = i.normal;
            // result.roughness = 1.0;
        }
    }

    if box_intersect_test(vec3f(-12.0, -1.001, -12.0), vec3f(12.0, -0.999, 12.0), ray) {
        let i = plane_intersect_check(vec3f(0.0, -1.0, 0.0), vec3f(0.0, 1.0, 0.0), ray);

        if i.is_hit && i.distance < result.distance {
            result.is_hit = true;

            result.distance = i.distance;
            result.emission = vec3f(0.0, 0.0, 0.0);
            result.color = vec3f(0.8, 0.4, 0.4);
            result.normal = vec3f(0.0, 1.0, 0.0);
            // result.roughness = 0.001;
        }
    }

    {
        let i = box_intersect_check(vec3f(0.0, 0.0, 0.0), vec3f(1.0, -1.0, 1.0), ray);

        if i.is_hit && i.distance < result.distance {
            result.is_hit = true;
            result.distance = i.distance;
            result.color = vec3f(0.8, 0.8, 0.8);
            result.emission = vec3f(0.0, 0.0, 0.0);
            result.normal = i.normal;
            // result.roughness = 0.001;
        }
    }

    return result;
}

const MAX_BOUNCE: u32 = 8;

fn trace(init_ray: Ray) -> vec3f {
    var ray_color = vec3f(1.0, 1.0, 1.0);
    var incoming_light = vec3f(0.0, 0.0, 0.0);
    var ray = init_ray;

    var index = MAX_BOUNCE + 1;

    while index > 0 {
        let result = intersect_scene(ray);

        if !result.is_hit {
            break;
        }

        incoming_light += result.emission * ray_color;
        ray.origin += ray.direction * result.distance + result.normal * 0.001;
        ray.direction = rand_vec3();
        ray.direction *= sign(dot(ray.direction, result.normal));
        ray_color *= result.color * clamp(dot(result.normal, ray.direction), 0.0, 1.0) * 3.14159265358979;

        index = index - 1;
    }

    return incoming_light;
}

fn tex_coord_to_ray(tex_coord: vec2f) -> Ray {
    let coord = tex_coord * 2.0 - 1.0;
    var ray: Ray;
    ray.origin = camera.location;
    ray.direction = normalize(camera.direction * camera.near + camera.right * camera.projection_width * coord.x + camera.up * camera.projection_height * coord.y);
    return ray;
}

@fragment
fn fs_main(@builtin(position) frag_coord_4f: vec4f, @location(0) tex_coord: vec2f) -> @location(0) vec4f {
    _rand_seed = u32(tex_coord.x * 3123456.0) * u32(tex_coord.y * 8765345.0) * u32((cos(system.time) + 1.123123) * 324234234.5);

    let out_color = (
        trace(tex_coord_to_ray(tex_coord + system.texel_size * vec2f(rand_f32(), rand_f32()))) +
        trace(tex_coord_to_ray(tex_coord + system.texel_size * vec2f(rand_f32(), rand_f32()))) +
        trace(tex_coord_to_ray(tex_coord + system.texel_size * vec2f(rand_f32(), rand_f32()))) +
        trace(tex_coord_to_ray(tex_coord + system.texel_size * vec2f(rand_f32(), rand_f32())))
    ) / 4.0;

    return vec4f(textureLoad(read_collector, vec2i(frag_coord_4f.xy), 0).xyz * f32(system.static_frame_index != 0) + out_color, 0.0);
} // fn fs_main

// file shader.wgsl
