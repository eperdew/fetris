#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct OverlayUniforms {
    frame_parity: f32,
    hue_shift: f32,
    overlay_opacity: f32,
    _pad: f32,
}

@group(2) @binding(0) var<uniform> uniforms: OverlayUniforms;
@group(2) @binding(1) var overlay_texture: texture_2d<f32>;
@group(2) @binding(2) var overlay_sampler: sampler;

fn hue_rotate(col: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return vec3<f32>(
        dot(col, vec3<f32>(0.299 + 0.701*c + 0.168*s, 0.587 - 0.587*c + 0.330*s, 0.114 - 0.114*c - 0.497*s)),
        dot(col, vec3<f32>(0.299 - 0.299*c - 0.328*s, 0.587 + 0.413*c + 0.035*s, 0.114 - 0.114*c + 0.292*s)),
        dot(col, vec3<f32>(0.299 - 0.300*c + 1.250*s, 0.587 - 0.588*c - 1.050*s, 0.114 + 0.886*c - 0.203*s))
    );
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    if (floor(in.position.y) % 2.0) != uniforms.frame_parity {
        discard;
    }
    // UVs from the Rectangle mesh are vertically flipped relative to the render target
    // because the camera Y-flip makes world bottom-left appear at screen top-left.
    let uv = vec2<f32>(in.uv.x, 1.0 - in.uv.y);
    var tex = textureSample(overlay_texture, overlay_sampler, uv);
    if uniforms.hue_shift > 0.001 {
        tex = vec4<f32>(hue_rotate(tex.rgb, uniforms.hue_shift * 6.28318), tex.a);
    }
    tex.a *= uniforms.overlay_opacity;
    return tex;
}
