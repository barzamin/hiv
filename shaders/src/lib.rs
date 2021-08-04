#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr, lang_items),
    register_attr(spirv)
)]
#![deny(warnings)] // HACK(via:eddyb) warnings not easily exposed from spirv-builder otherwise

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

#[cfg(target_arch = "spirv")] // if building no_std
#[allow(unused_imports)]
use spirv_std::num_traits::Float;

#[allow(unused_imports)]
use spirv_std::glam::{vec2, vec3, vec4, Vec2, Vec3, Vec4};

use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct ShaderConstants {
    pub width_px: u32,
    pub height_px: u32,

    pub time: f32,
}

pub fn scene(pos: Vec3) -> f32 {
    (pos - vec3(0., 0., 10.)).length() - 3.0
}

const STEP_CNT: u32 = 16;
const THRESH: f32 = 0.001;
pub fn raycast(p0: Vec3, ray: Vec3) -> (u32, f32) {
    let mut t = 0f32;
    for i in 0..STEP_CNT {
        let dist = scene(p0 + ray * t);
        if dist <= THRESH * t {
            return (i, t);
        }

        t += dist;
    }

    (STEP_CNT, -1.0)
}

fn render(p0: Vec3, ray: Vec3) -> Vec3 {
    let (_iters, t) = raycast(p0, ray);

    Vec3::splat(1. - t * 0.075)
}

fn screen2uv(fragcoord: Vec4, ssize_px: Vec2) -> Vec2 {
    let frag_coord = fragcoord.truncate().truncate();
    let mut uv = 2.0 * frag_coord / ssize_px - Vec2::splat(1.);
    uv.x *= ssize_px.x as f32 / ssize_px.y as f32;
    uv
}

struct Camera {
    pos: Vec3,
    #[allow(dead_code)]
    tgt: Vec3,
    persp: f32,

    fwd: Vec3,
    rgt: Vec3,
    up:  Vec3,
}

impl Camera {
    pub fn new_pointing_at(pos: Vec3, tgt: Vec3, persp: f32) -> Self {
        let fwd = (tgt - pos).normalize();
        // crossing the forward direction with world up gives right
        let rgt = vec3(0., 1., 0.).cross(fwd).normalize();
        let up = fwd.cross(rgt).normalize();


        Camera {
            pos, tgt, persp,
            fwd, rgt, up,
        }
    }

    pub fn ray(&self, uv: Vec2) -> Vec3 {
        (uv.x * self.rgt + uv.y * self.up + self.fwd * self.persp).normalize()
    }

    pub fn root(&self) -> Vec3 {
        self.pos
    }
}

#[allow(unused_variables)]
#[spirv(fragment)]
pub fn main_fs(
    #[spirv(frag_coord)] in_frag_coord: Vec4,
    #[spirv(push_constant)] constants: &ShaderConstants,
    output: &mut Vec4,
) {
    let ssize = vec2(constants.width_px as f32, constants.height_px as f32);
    let uv = screen2uv(in_frag_coord, ssize);

    let cam = Camera::new_pointing_at(vec3(0., 0., -1.), vec3(0., 0., 0.), 2.);

    *output = render(cam.root(), cam.ray(uv)).extend(1.);
}

#[spirv(vertex)]
pub fn main_vs(#[spirv(vertex_index)] vert_idx: i32, #[spirv(position)] pos_out: &mut Vec4) {
    // cover the screen with a single tri as per [https://www.saschawillems.de/blog/2016/08/13/vulkan-tutorial-on-rendering-a-fullscreen-quad-without-buffers/]
    let uv = vec2(((vert_idx << 1) & 2) as f32, (vert_idx & 2) as f32);
    let pos = 2.0 * uv - Vec2::ONE;

    *pos_out = pos.extend(0.0).extend(1.0);
}
