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
use spirv_std::num_traits::Float;

#[allow(unused_imports)]
use spirv_std::glam::{vec2, vec4, Vec2, Vec4};

use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct ShaderConstants {
    pub width_px: u32,
    pub height_px: u32,

    pub time: f32,
}

#[allow(unused_variables)]
#[spirv(fragment)]
pub fn main_fs(
    #[spirv(frag_coord)] in_frag_coord: Vec4,
    #[spirv(push_constant)] constants: &ShaderConstants,
    output: &mut Vec4,
) {
    let frag_coord = in_frag_coord.truncate().truncate();

    *output = vec4(constants.time.cos()*0.5 + 0.5, 0., 0., 1.);
}

#[spirv(vertex)]
pub fn main_vs(#[spirv(vertex_index)] vert_idx: i32, #[spirv(position)] pos_out: &mut Vec4) {
    // cover the screen with a single tri as per [https://www.saschawillems.de/blog/2016/08/13/vulkan-tutorial-on-rendering-a-fullscreen-quad-without-buffers/]
    let uv = vec2(((vert_idx << 1) & 2) as f32, (vert_idx & 2) as f32);
    let pos = 2.0 * uv - Vec2::ONE;

    *pos_out = pos.extend(0.0).extend(1.0);
}
