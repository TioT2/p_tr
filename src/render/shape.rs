//! Scene-related implementation module

use crate::math::Vec3f;

/// Solid material structure
#[derive(Copy, Clone)]
pub struct Material {
    /// Base color (in \[0; 1\])
    pub base_color: Vec3f,

    /// How rough object is (in \[0; 1\])
    pub roughness: f32,

    /// Amount of outgoing light (in \[0; +inf\))
    pub emission: Vec3f,

    /// How metallic shape is (in \[0; 1\])
    pub metallic: f32,
}

/// Material codegen wrapper
struct CgMaterial(Material);

impl std::fmt::Display for CgMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "Material(vec3f({}, {}, {}), {}, vec3f({}, {}, {}), {})",
            self.0.base_color.x,
            self.0.base_color.y,
            self.0.base_color.z,
            self.0.roughness,
            self.0.emission.x,
            self.0.emission.y,
            self.0.emission.z,
            self.0.metallic,
        )
    }
}

/// Codegen vec3f
struct CgVec3f(Vec3f);

impl std::fmt::Display for CgVec3f {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "vec3f({}, {}, {})", self.0.x, self.0.y, self.0.z)
    }
}

/// Codegen result
struct CgResult(u32);

impl std::fmt::Display for CgResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "result{}", self.0)
    }
}

/// Shape trait
pub trait Shape {
    /// Generate intersection with this object
    fn gen_intersection(&self, out_result_index: u32, out: &mut dyn std::io::Write) -> std::io::Result<()>;
}

/// Scene structure, generic shape
pub struct Scene {
    /// Set of the scene shapes
    shapes: Vec<Box<dyn Shape>>,
}

impl Scene {
    /// Scene constructor
    pub fn new() -> Self {
        Self {
            shapes: Vec::new(),
        }
    }

    /// Add shape to scene
    pub fn add_shape(&mut self, shape: Box<dyn Shape>) {
        self.shapes.push(shape);
    }
}

impl Shape for Scene {
    fn gen_intersection(&self, out_result_index: u32, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(out, "{{ // Scene")?;

        for shape in &self.shapes {
            shape.gen_intersection(out_result_index, out)?;
        }

        writeln!(out, "}}")?;

        Ok(())
    }
}

/// Sphere structure
pub struct Sphere {
    /// Center
    center: Vec3f,

    /// Radius (non-negative)
    radius: f32,

    /// Sphere material
    material: Material,
}

impl Sphere {
    /// Sphere constructor
    pub fn new(center: Vec3f, radius: f32, material: Material) -> Self {
        Self { center, radius, material }
    }
}

impl Shape for Sphere {
    /// Generate sphere-scene intersection
    fn gen_intersection(&self, out_result_index: u32, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(out,
            "\n\
            {{ // Sphere \n\
            let delta = {center} - ray.origin; \n\
            let delta_proj_len = dot(delta, ray.direction); \n\
            let delta_proj = ray.direction * delta_proj_len; \n\
            let h = distance(delta, delta_proj); \n\
            let d = sqrt({radius} * {radius} - h * h); \n\
            let distance = delta_proj_len - d; \n\
            \n\
            if ((delta_proj_len > 0.0 && h <= {radius}) && distance < {result}.distance) {{\n\
                // Mix with the actual result\n\
                {result}.is_hit = true; \n\
                {result}.distance = distance; \n\
                {result}.normal = (delta_proj - delta - ray.direction * d) / {radius}; \n\
                {result}.material = {material}; \n\
            }}\n\
            }}\n\
            ",
            radius = self.radius,
            result = CgResult(out_result_index),
            material = CgMaterial(self.material),
            center = CgVec3f(self.center),
        )
    }
}

/// Quad structure
pub struct Quad {
    base: Vec3f,
    u: Vec3f,
    v: Vec3f,
    material: Material,
}

impl Quad {
    pub fn new(base: Vec3f, u: Vec3f, v: Vec3f, material: Material) -> Self {
        Self { base, u, v, material }
    }
}

impl Shape for Quad {
    fn gen_intersection(&self, out_result_index: u32, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        let normal = self.u.cross(self.v).normalized();

        writeln!(out,
            "{{ // Quad \n\
            let distance = dot({base} - ray.origin, {normal}) / dot({normal}, ray.direction); \n\
            let location = ray.direction * distance + ray.origin; \n\
            let uv = vec2f(dot(location - {base}, {u}), dot(location - {base}, {v})); \n\
            \n\
            let is_hit = distance > 0.0 \n\
                && uv.x > 0.0 && uv.y > 0.0 \n\
                && uv.x < 1.0 && uv.y < 1.0; \n\
            \n\
            if (is_hit && distance < {result}.distance) {{\n\
                {result}.is_hit = true; \n\
                {result}.distance = distance; \n\
                {result}.normal = {normal}; \n\
                {result}.material = {material}; \n\
            }}\n\
            }}\n\
            ",
            result = CgResult(out_result_index),
            base = CgVec3f(self.base),
            u = CgVec3f(self.u),
            v = CgVec3f(self.v),
            normal = CgVec3f(normal),
            material = CgMaterial(self.material),
        )
    }
}

/// Axis-aligned bound box
pub struct Aabb {
    min: Vec3f,
    max: Vec3f,
    material: Material,
}

impl Aabb {
    pub fn new(v0: Vec3f, v1: Vec3f, material: Material) -> Self {
        let mm = |f: &dyn Fn(f32, f32) -> f32| Vec3f::new(f(v0.x, v1.x), f(v0.y, v1.y), f(v0.z, v1.z));
        Self { min: mm(&f32::min), max: mm(&f32::max), material }
    }
}

impl Shape for Aabb {
    fn gen_intersection(&self, out_result_index: u32, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(out,
            "{{ // Aabb \n\
            let utv0 = ({p0} - ray.origin) / ray.direction; \n\
            let utv1 = ({p1} - ray.origin) / ray.direction; \n\
            let tv0 = min(utv0, utv1); \n\
            let tv1 = max(utv0, utv1); \n\
            let t_near = max(max(tv0.x, tv0.y), tv0.z); \n\
            let t_far = min(min(tv1.x, tv1.y), tv1.z); \n\
            let distance = select(t_far, t_near, t_near > 0.0); \n\
            let is_hit = t_far >= max(t_near, 0.0); \n\
            \n\
            if (is_hit && distance < {result}.distance) {{ \n\
                {result}.is_hit = true;
                {result}.distance = distance; \n\
                {result}.normal = vec3f(tv0 == vec3f(t_near)) * -sign(ray.direction); \n\
                {result}.material = {material}; \n\
            }}\n\
            }}\n\
            ",
            result = CgResult(out_result_index),
            p0 = CgVec3f(self.min),
            p1 = CgVec3f(self.max),
            material = CgMaterial(self.material)
        )
    }
}
