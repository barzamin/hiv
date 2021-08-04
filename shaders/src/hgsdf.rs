#![allow(dead_code)]
#[cfg(target_arch = "spirv")] // if building no_std
#[allow(unused_imports)]
use spirv_std::num_traits::Float;

#[allow(unused_imports)]
use spirv_std::glam::{vec2, vec3, vec4, Vec2, Vec3, Vec4};
use spirv_std::glam::swizzles::*;

pub fn f_sphere(p: Vec3, r: f32) -> f32 {
    p.length() - r
}

pub fn f_cylinder(p: Vec3, r: f32, h: f32) -> f32 {
    let d = p.xz().length() - r;
    d.max(p.y.abs() - h)
}
pub fn f_torus(p: Vec3, r_minor: f32, r_major: f32) -> f32 {
    vec2(p.xz().length() - r_major, p.y).length() - r_minor
}