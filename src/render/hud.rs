use bevy::prelude::*;
use crate::render::{BAR_WIDTH, BAR_X, BOARD_BG, BOARD_X, BOARD_Y, CELL, DIVIDER_X, SIDEBAR_X};
use crate::render::assets::GameAssets;
use crate::data::{BOARD_ROWS, Grade};

#[derive(Component, Clone, Copy)]
pub struct HudNode;

pub fn render_hud(
    mut commands: Commands,
    existing: Query<Entity, With<HudNode>>,
    judge: Res<crate::judge::Judge>,
    progress: Res<crate::resources::GameProgress>,
    assets: Res<GameAssets>,
    mut clear_color: ResMut<ClearColor>,
) {
    for e in &existing {
        commands.entity(e).despawn();
    }

    *clear_color = ClearColor(grade_bg_color(judge.grade().index()));

    spawn_grade_bar(&mut commands, judge.score(), judge.grade());

    let dim = Color::srgba(0.5, 0.5, 0.5, 1.0);
    const FONT_LG: f32 = 26.0;
    const FONT_SM: f32 = 18.0;
    const LH: f32 = 30.0;

    let x = SIDEBAR_X;
    let mut y = BOARD_Y + 22.0;

    macro_rules! push {
        ($text:expr, $size:expr, $color:expr) => {
            commands.spawn((
                HudNode,
                Text2d::new($text),
                TextFont { font: assets.font.clone(), font_size: $size, ..default() },
                TextColor($color),
                bevy::sprite::Anchor::TOP_LEFT,
                Transform::from_xyz(x, y, 10.0),
            ));
        };
    }

    push!("LEVEL".to_string(), FONT_SM, dim); y += LH;
    push!(format!("{:03}", progress.level), FONT_LG, Color::WHITE); y += 6.0;
    commands.spawn((
        HudNode,
        Sprite {
            color: dim,
            custom_size: Some(Vec2::new(48.0, 2.0)),
            ..default()
        },
        bevy::sprite::Anchor::TOP_LEFT,
        Transform::from_xyz(x, y, 10.0),
    ));
    y += 24.0;
    push!(format!("{}", next_level_barrier(progress.level)), FONT_LG, Color::WHITE); y += LH + 8.0;

    push!("LINES".to_string(), FONT_SM, dim); y += LH;
    push!(format!("{}", progress.lines), FONT_LG, Color::WHITE); y += LH + 8.0;

    push!("TIME".to_string(), FONT_SM, dim); y += LH;
    push!(format_time(progress.ticks_elapsed), FONT_LG, Color::WHITE); y += LH + 8.0;

    push!("SCORE".to_string(), FONT_SM, dim); y += LH;
    push!(format!("{}", judge.score()), FONT_LG, Color::WHITE); y += LH + 8.0;

    push!("GRADE".to_string(), FONT_SM, dim); y += LH;
    push!(format!("{}", judge.grade()), FONT_LG, Color::WHITE); y += LH + 8.0;

    push!("NEXT".to_string(), FONT_SM, dim); y += LH;
    let (_, next_opt) = Grade::grade_progress(judge.score());
    let next_str = match next_opt {
        Some(n) => format!("{}", n),
        None => "??????".to_string(),
    };
    push!(next_str, FONT_LG, Color::WHITE);
}

fn spawn_grade_bar(commands: &mut Commands, score: u32, grade: Grade) {
    let (prev, next_opt) = Grade::grade_progress(score);
    let progress: f32 = match next_opt {
        None => 1.0,
        Some(next) => (score - prev) as f32 / (next - prev) as f32,
    };

    let bar_h = BOARD_ROWS as f32 * CELL;
    const SHADOW_PAD: f32 = 2.0;
    let inner_h = bar_h - SHADOW_PAD * 2.0;
    let fill_h = inner_h * progress;

    commands.spawn((HudNode, Sprite {
        color: Color::srgba(0.0, 0.0, 0.0, 0.55),
        custom_size: Some(Vec2::new(BAR_WIDTH + SHADOW_PAD * 2.0, bar_h)),
        ..default()
    }, bevy::sprite::Anchor::TOP_LEFT, Transform::from_xyz(BAR_X - SHADOW_PAD, BOARD_Y, 5.0)));

    commands.spawn((HudNode, Sprite {
        color: Color::srgba(0.25, 0.25, 0.35, 1.0),
        custom_size: Some(Vec2::new(1.5, bar_h)),
        ..default()
    }, bevy::sprite::Anchor::TOP_LEFT, Transform::from_xyz(DIVIDER_X, BOARD_Y, 5.0)));

    commands.spawn((HudNode, Sprite {
        color: BOARD_BG,
        custom_size: Some(Vec2::new(BAR_WIDTH, inner_h)),
        ..default()
    }, bevy::sprite::Anchor::TOP_LEFT, Transform::from_xyz(BAR_X, BOARD_Y + SHADOW_PAD, 6.0)));

    commands.spawn((HudNode, Sprite {
        color: grade_bar_color(grade.index()),
        custom_size: Some(Vec2::new(BAR_WIDTH, fill_h)),
        ..default()
    }, bevy::sprite::Anchor::TOP_LEFT, Transform::from_xyz(BAR_X, BOARD_Y + SHADOW_PAD + inner_h - fill_h, 7.0)));
}

fn grade_bar_color(idx: usize) -> Color {
    match idx % 7 {
        0 => Color::srgba_u8(220, 50, 50, 200),
        1 => Color::srgba_u8(230, 130, 0, 200),
        2 => Color::srgba_u8(220, 210, 0, 200),
        3 => Color::srgba_u8(50, 180, 50, 200),
        4 => Color::srgba_u8(50, 100, 220, 200),
        5 => Color::srgba_u8(80, 0, 200, 200),
        _ => Color::srgba_u8(150, 0, 220, 200),
    }
}

fn grade_bg_color(idx: usize) -> Color {
    let tint = grade_bar_color(idx).to_srgba();
    Color::srgba(0.04 + tint.red * 0.14, 0.04 + tint.green * 0.14, 0.07 + tint.blue * 0.14, 1.0)
}

fn next_level_barrier(level: u32) -> u32 {
    let round_up = (level + 1).next_multiple_of(100);
    if round_up == 1000 { 999 } else { round_up }
}

pub fn format_time(ticks: u64) -> String {
    let seconds = ticks / 60;
    let ms = (ticks % 60) * 1000 / 60;
    let mm = seconds / 60;
    let ss = seconds % 60;
    format!("{:02}:{:02}.{:03}", mm, ss, ms)
}

