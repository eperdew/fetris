use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

pub mod assets;
pub mod board;
pub mod hud;
pub mod overlay_material;
pub mod overlays;
pub mod particles;
pub mod piece;

pub const CELL: f32 = 32.0;
pub const INSET: f32 = 2.0;
pub const PAD: f32 = 20.0;
pub const BOARD_X: f32 = PAD;
pub const BOARD_Y: f32 = 2.0 * CELL + 2.0 * PAD;
pub const BAR_WIDTH: f32 = 24.0;
pub const BAR_LEFT_GAP: f32 = 24.0;
pub const BAR_RIGHT_GAP: f32 = 14.0;
pub const BAR_X: f32 = BOARD_X + crate::data::BOARD_COLS as f32 * CELL + BAR_LEFT_GAP;
pub const SIDEBAR_X: f32 = BAR_X + BAR_WIDTH + BAR_RIGHT_GAP;
pub const DIVIDER_X: f32 = BOARD_X + crate::data::BOARD_COLS as f32 * CELL + BAR_LEFT_GAP / 2.0;
pub const WINDOW_W: f32 = 560.0;
pub const WINDOW_H: f32 = 780.0;
pub const BOARD_BG: Color = Color::srgba(0.06, 0.06, 0.10, 1.0);
pub const OVERLAY_RENDER_LAYER: usize = 1;

#[derive(Resource)]
pub struct OverlayRenderSetup {
    pub material: Handle<overlay_material::OverlayMaterial>,
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        use crate::app_state::AppState;
        use crate::systems::active::active_phase_system;
        use crate::systems::line_clear_delay::line_clear_delay_system;
        app.add_plugins(Material2dPlugin::<overlay_material::OverlayMaterial>::default());
        app.add_systems(Startup, assets::load_assets);
        app.add_systems(Startup, setup_overlay_camera);
        app.add_systems(
            Update,
            board::render_board.run_if(
                in_state(AppState::Playing)
                    .or(in_state(AppState::GameOver))
                    .or(in_state(AppState::Ready)),
            ),
        );
        app.add_systems(
            Update,
            (piece::render_active_piece, piece::render_next_preview)
                .run_if(in_state(AppState::Playing).or(in_state(AppState::GameOver))),
        );
        app.add_systems(
            Update,
            hud::render_hud.run_if(in_state(AppState::Playing).or(in_state(AppState::GameOver))),
        );
        app.add_systems(
            FixedUpdate,
            particles::spawn_particles_on_line_clear
                .after(active_phase_system)
                .before(line_clear_delay_system)
                .run_if(in_state(AppState::Playing)),
        );
        app.add_systems(FixedUpdate, particles::update_particles);
        app.add_systems(
            Update,
            (
                overlays::spawn_line_clear_overlay,
                overlays::render_state_text,
            )
                .run_if(
                    in_state(AppState::Playing)
                        .or(in_state(AppState::Ready))
                        .or(in_state(AppState::GameOver)),
                ),
        );
        app.add_systems(FixedUpdate, overlays::tick_line_clear_overlay);
        app.add_systems(
            Update,
            update_overlay_material.run_if(
                in_state(AppState::Playing)
                    .or(in_state(AppState::Ready))
                    .or(in_state(AppState::GameOver)),
            ),
        );
    }
}

fn setup_overlay_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut overlay_materials: ResMut<Assets<overlay_material::OverlayMaterial>>,
) {
    use bevy::camera::{RenderTarget, ScalingMode};
    use bevy::render::render_resource::TextureFormat;

    let image_handle = images.add(Image::new_target_texture(
        WINDOW_W as u32,
        WINDOW_H as u32,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    ));

    let mut projection = OrthographicProjection::default_2d();
    projection.scaling_mode = ScalingMode::Fixed {
        width: WINDOW_W,
        height: WINDOW_H,
    };
    projection.viewport_origin = Vec2::new(0.0, 1.0);
    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            clear_color: Color::srgba(0.0, 0.0, 0.0, 0.0).into(),
            ..default()
        },
        RenderTarget::Image(image_handle.clone().into()),
        Projection::Orthographic(projection),
        Transform::from_scale(Vec3::new(1.0, -1.0, 1.0)),
        RenderLayers::layer(OVERLAY_RENDER_LAYER),
    ));

    let material = overlay_materials.add(overlay_material::OverlayMaterial {
        uniforms: overlay_material::OverlayUniforms::default(),
        texture: image_handle,
    });

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(WINDOW_W, WINDOW_H))),
        MeshMaterial2d(material.clone()),
        Transform::from_xyz(WINDOW_W * 0.5, WINDOW_H * 0.5, 190.0),
    ));

    commands.insert_resource(OverlayRenderSetup { material });
}

fn update_overlay_material(
    overlay_setup: Res<OverlayRenderSetup>,
    mut overlay_materials: ResMut<Assets<overlay_material::OverlayMaterial>>,
    q: Query<&overlays::LineClearOverlay>,
    progress: Res<crate::resources::GameProgress>,
) {
    let Some(mat) = overlay_materials.get_mut(&overlay_setup.material) else {
        return;
    };
    match q.single() {
        Ok(overlay) => {
            mat.uniforms.frame_parity = (progress.ticks_elapsed % 2) as f32;
            mat.uniforms.overlay_opacity = overlays::overlay_opacity(overlay.kind);
            mat.uniforms.hue_shift =
                overlays::overlay_hue_shift(overlay.kind, progress.ticks_elapsed);
        }
        Err(_) => {
            mat.uniforms.overlay_opacity = 0.0;
        }
    }
}

pub fn piece_color(kind: crate::data::PieceKind) -> Color {
    use crate::data::PieceKind;
    match kind {
        PieceKind::I => Color::srgba_u8(200, 50, 50, 255),
        PieceKind::O => Color::srgba_u8(220, 200, 0, 255),
        PieceKind::T => Color::srgba_u8(0, 200, 200, 255),
        PieceKind::S => Color::srgba_u8(200, 0, 200, 255),
        PieceKind::Z => Color::srgba_u8(0, 160, 0, 255),
        PieceKind::J => Color::srgba_u8(50, 100, 220, 255),
        PieceKind::L => Color::srgba_u8(255, 150, 100, 255),
    }
}

/// Returns a bundle: (Sprite with image+color+custom_size, Anchor::BOTTOM_LEFT, Transform).
/// In Bevy 0.18, Anchor is a separate Component (not a field on Sprite).
/// BOTTOM_LEFT is the visual top-left in our Y-flip camera (positive Y = down).
pub fn cell_sprite(x: f32, y: f32, color: Color, texture: Handle<Image>, z: f32) -> impl Bundle {
    (
        Sprite {
            image: texture,
            color,
            custom_size: Some(Vec2::new(CELL - INSET * 2.0, CELL - INSET * 2.0)),
            ..default()
        },
        bevy::sprite::Anchor::BOTTOM_LEFT,
        Transform::from_xyz(x + INSET, y + INSET, z),
    )
}
