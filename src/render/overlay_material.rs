use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d};

#[derive(ShaderType, Clone)]
pub struct OverlayUniforms {
    pub frame_parity: f32,
    pub hue_shift: f32,
    pub overlay_opacity: f32,
    pub pixel_scale: f32,
}

impl Default for OverlayUniforms {
    fn default() -> Self {
        Self {
            frame_parity: 0.0,
            hue_shift: 0.0,
            overlay_opacity: 0.0,
            pixel_scale: 1.0,
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct OverlayMaterial {
    #[uniform(0)]
    pub uniforms: OverlayUniforms,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
}

impl Material2d for OverlayMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/overlay.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}
