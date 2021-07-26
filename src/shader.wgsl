struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
};

[[stage(vertex)]]
fn main(
    [[builtin(vertex_index)]] in_vertex_idx: u32,
) -> VertexOutput {
    var quad = array<vec4<f32>, 6>(
        vec4<f32>(1., -1., 0., 1.),
        vec4<f32>(-1., -1., 0., 1.),
        vec4<f32>(0., 1., 0., 1.),
        vec4<f32>(1., 0., 0., 1.),
        vec4<f32>(1., 0., 0., 1.),
        vec4<f32>(1., 0., 0., 1.)
    );

    var out: VertexOutput;
    out.clip_position = quad[in_vertex_idx];
    return out;
}

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}