[[block]]
struct Uniforms {
    width_px:  u32;
    height_px: u32;

    time: f32;
};

[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

[[stage(vertex)]]
fn main_vs(
    [[builtin(vertex_index)]] vert_idx: u32,
) -> [[builtin(position)]] vec4<f32> {
    let uv = vec2<f32>(f32((vert_idx << 1u) & 2u), f32(vert_idx & 2u));
    let pos = 2.0 * uv - vec2<f32>(1., 1.);

    return vec4<f32>(pos, 0., 1.);
}

fn scene(pos: vec3<f32>) -> f32 {
    return length(pos - vec3<f32>(0., 0., 10.)) - 3.;
}

let MAX_ITERS: u32 = 64u;
let DIST_THRESH: f32 = 0.0001;
struct CastResult {
    iters: u32;
    dist: f32;
};
fn raycast(p0: vec3<f32>, ray: vec3<f32>) -> CastResult {
    var p = p0;
    for (var iter: u32 = 0u; iter < MAX_ITERS; iter = iter + 1u) {
        let dist = scene(p);
        if (dist < DIST_THRESH) {
            return CastResult(iter, dist);
        }
        p = p + ray * dist;
    }

    return CastResult(MAX_ITERS, -1.0);
}

fn render(p0: vec3<f32>, ray: vec3<f32>) -> vec3<f32> {
    let r = raycast(p0, ray);

    return vec3<f32>(f32(r.iters)/f32(MAX_ITERS));
}

[[stage(fragment)]]
fn main_fs([[builtin(position)]] pos: vec4<f32>) -> [[location(0)]] vec4<f32> {
    let uv = (pos.xy - 0.5 * vec2<f32>(f32(uniforms.width_px), f32(uniforms.height_px)))
        / f32(uniforms.height_px);
    
    let cam_pos = vec3<f32>(0., 0., -1.);
    let cam_tgt = vec3<f32>(0., 0., 0.);
    let cam_fwd = normalize(cam_tgt - cam_pos);
    let cam_rgt = normalize(cross(vec3<f32>(0., 1., 0.), cam_fwd));
    let cam_up  = normalize(cross(cam_fwd, cam_rgt));
    let cam_persp = 2.;
    let ray_dir = normalize(uv.x * cam_rgt + uv.y * cam_up + cam_fwd * cam_persp);

    return vec4<f32>(render(cam_pos, ray_dir), 1.0);
}