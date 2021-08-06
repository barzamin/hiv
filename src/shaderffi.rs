use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct ShaderConstants {
    pub width_px: u32,
    pub height_px: u32,

    pub time: f32,
}
