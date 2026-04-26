use bevy::prelude::*;

#[derive(Resource)]
pub struct GameAssets {
    pub font: Handle<Font>,
    pub cell_texture: Handle<Image>,
}

pub fn load_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    let font = asset_server.load("font/Oxanium-Regular.ttf");
    let cell_texture = images.add(make_cell_image());
    commands.insert_resource(GameAssets { font, cell_texture });
}

fn make_cell_image() -> Image {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    const SIZE: u32 = 32;
    let mut pixels = vec![255u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            // Camera Y-flip: y=0 in image appears at visual bottom; y=SIZE-1 at visual top.
            // Bright highlight should be at visual top-left, so use the flipped coordinate.
            let y_flipped = SIZE - 1 - y;
            let fy = y_flipped as f32 / (SIZE - 1) as f32;
            let raw = if x == 0 || y_flipped == SIZE - 1 {
                1.0
            } else {
                1.0 - 0.4 * fy
            };
            let quantized = (raw * 16.0).floor() / 16.0;
            let v = (quantized * 255.0) as u8;
            let i = ((y * SIZE + x) * 4) as usize;
            pixels[i] = v;
            pixels[i + 1] = v;
            pixels[i + 2] = v;
        }
    }
    Image::new(
        Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}
