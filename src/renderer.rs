use crate::menu::Menu;
use crate::types::{
    BOARD_COLS, BOARD_ROWS, GameEvent, GameMode, GameSnapshot, Grade, Kind, MenuScreen, PieceKind,
};
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams};
use macroquad::prelude::*;

const OVERLAY_VERTEX_SHADER: &str = r#"
    #version 100
    attribute vec3 position;
    attribute vec2 texcoord;
    attribute vec4 color0;
    varying vec2 uv;
    varying vec4 color;
    uniform mat4 Model;
    uniform mat4 Projection;
    void main() {
        gl_Position = Projection * Model * vec4(position, 1.0);
        color = color0 / 255.0;
        uv = texcoord;
    }
"#;

const OVERLAY_FRAGMENT_SHADER: &str = r#"
    #version 100
    precision mediump float;
    varying vec2 uv;
    varying vec4 color;
    uniform sampler2D Texture;
    uniform float frame_parity;
    uniform float hue_shift;
    uniform float overlay_opacity;

    vec3 hue_rotate(vec3 col, float angle) {
        float c = cos(angle);
        float s = sin(angle);
        return vec3(
            dot(col, vec3(0.299 + 0.701*c + 0.168*s,
                          0.587 - 0.587*c + 0.330*s,
                          0.114 - 0.114*c - 0.497*s)),
            dot(col, vec3(0.299 - 0.299*c - 0.328*s,
                          0.587 + 0.413*c + 0.035*s,
                          0.114 - 0.114*c + 0.292*s)),
            dot(col, vec3(0.299 - 0.300*c + 1.250*s,
                          0.587 - 0.588*c - 1.050*s,
                          0.114 + 0.886*c - 0.203*s))
        );
    }

    void main() {
        if (mod(floor(gl_FragCoord.y), 2.0) != frame_parity) {
            discard;
        }
        vec4 tex = texture2D(Texture, uv) * color;
        if (hue_shift > 0.001) {
            tex.rgb = hue_rotate(tex.rgb, hue_shift * 6.28318);
        }
        tex.a *= overlay_opacity;
        gl_FragColor = tex;
    }
"#;

const OVERLAY_LIFETIME: u32 = 90;

#[derive(Debug)]
enum OverlayKind {
    Double,
    Triple,
    Fetris,
}

#[derive(Debug)]
struct LineClearOverlay {
    kind: OverlayKind,
    frames_remaining: u32,
}

impl LineClearOverlay {
    fn label(&self) -> &'static str {
        match self.kind {
            OverlayKind::Double => "DOUBLE",
            OverlayKind::Triple => "TRIPLE",
            OverlayKind::Fetris => "FETRIS",
        }
    }

    fn opacity(&self) -> f32 {
        match self.kind {
            OverlayKind::Double => 0.45,
            OverlayKind::Triple => 0.75,
            OverlayKind::Fetris => 1.0,
        }
    }

    fn hue_shift(&self, ticks_elapsed: u64) -> f32 {
        match self.kind {
            OverlayKind::Fetris => (ticks_elapsed as f32 * 0.03) % 1.0,
            _ => 0.0,
        }
    }
}

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    age: u32,
    lifetime: u32,
    color: Color,
}

fn rand_f32() -> f32 {
    macroquad::rand::rand() as f32 / u32::MAX as f32
}

const CELL: f32 = 32.0;
const INSET: f32 = 2.0;
const PAD: f32 = 20.0;
const BOARD_X: f32 = PAD;
const BOARD_Y: f32 = 2.0 * CELL + 2.0 * PAD;
const BAR_WIDTH: f32 = 24.0;
const BAR_LEFT_GAP: f32 = 24.0;
const BAR_RIGHT_GAP: f32 = 14.0;
const BAR_X: f32 = BOARD_X + BOARD_COLS as f32 * CELL + BAR_LEFT_GAP;
const SIDEBAR_X: f32 = BAR_X + BAR_WIDTH + BAR_RIGHT_GAP;
const DIVIDER_X: f32 = BOARD_X + BOARD_COLS as f32 * CELL + BAR_LEFT_GAP / 2.0;
const BOARD_BG: Color = Color::new(0.06, 0.06, 0.10, 1.0);
const WINDOW_W: f32 = 560.0;
const WINDOW_H: f32 = 780.0;

pub(crate) struct Renderer {
    cell_texture: Texture2D,
    font: Font,
    particles: Vec<Particle>,
    overlay: Option<LineClearOverlay>,
    overlay_target: RenderTarget,
    overlay_material: Material,
}

impl Renderer {
    pub fn new() -> Self {
        let font =
            load_ttf_font_from_bytes(include_bytes!("../assets/font/Oxanium-Regular.ttf")).unwrap();
        let overlay_target = render_target(WINDOW_W as u32, WINDOW_H as u32);
        overlay_target.texture.set_filter(FilterMode::Nearest);
        let overlay_material = load_material(
            ShaderSource::Glsl {
                vertex: OVERLAY_VERTEX_SHADER,
                fragment: OVERLAY_FRAGMENT_SHADER,
            },
            MaterialParams {
                pipeline_params: PipelineParams {
                    color_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                    )),
                    alpha_blend: Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::One,
                        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                    )),
                    ..Default::default()
                },
                uniforms: vec![
                    UniformDesc::new("frame_parity", UniformType::Float1),
                    UniformDesc::new("hue_shift", UniformType::Float1),
                    UniformDesc::new("overlay_opacity", UniformType::Float1),
                ],
                ..Default::default()
            },
        )
        .expect("overlay shader failed to compile");
        Self {
            cell_texture: make_cell_texture(),
            font,
            particles: Vec::new(),
            overlay: None,
            overlay_target,
            overlay_material,
        }
    }

    fn draw_text(&self, text: &str, x: f32, y: f32, font_size: f32, color: Color) {
        draw_text_ex(
            text,
            x,
            y,
            TextParams {
                font: Some(&self.font),
                font_size: font_size as u16,
                color,
                ..Default::default()
            },
        );
    }

    fn draw_centered(&self, text: &str, y: f32, font_size: f32, color: Color) {
        let dims = measure_text(text, Some(&self.font), font_size as u16, 1.0);
        self.draw_text(
            text,
            (screen_width() - dims.width) / 2.0,
            y,
            font_size,
            color,
        );
    }

    fn draw_centered_x(&self, text: &str, cx: f32, y: f32, font_size: f32, color: Color) {
        let dims = measure_text(text, Some(&self.font), font_size as u16, 1.0);
        self.draw_text(text, cx - dims.width / 2.0, y, font_size, color);
    }

    pub fn render(&mut self, snapshot: &GameSnapshot, events: &[GameEvent]) {
        // Process events: spawn particles.
        for event in events {
            match event {
                GameEvent::LineClear { count } => {
                    spawn_particles(
                        &mut self.particles,
                        &snapshot.board,
                        &snapshot.rows_pending_compaction,
                        *count,
                    );
                    self.overlay = match count {
                        2 => Some(LineClearOverlay {
                            kind: OverlayKind::Double,
                            frames_remaining: OVERLAY_LIFETIME,
                        }),
                        3 => Some(LineClearOverlay {
                            kind: OverlayKind::Triple,
                            frames_remaining: OVERLAY_LIFETIME,
                        }),
                        4 => Some(LineClearOverlay {
                            kind: OverlayKind::Fetris,
                            frames_remaining: OVERLAY_LIFETIME,
                        }),
                        _ => None,
                    };
                }
            }
        }

        self.update_particles();

        clear_background(grade_bg_color(snapshot.grade.index()));
        self.render_board(snapshot);
        self.render_particles();
        self.render_grade_bar(snapshot);
        self.render_sidebar(snapshot);
        self.render_overlay(snapshot);
        self.render_line_clear_overlay(snapshot.ticks_elapsed);
    }

    fn update_particles(&mut self) {
        use crate::constants::PARTICLE_GRAVITY;
        for p in &mut self.particles {
            p.x += p.vx;
            p.y += p.vy;
            p.vy += PARTICLE_GRAVITY;
            p.age += 1;
        }
        self.particles.retain(|p| p.age < p.lifetime);
    }

    fn render_particles(&self) {
        for p in &self.particles {
            let alpha = 1.0 - p.age as f32 / p.lifetime as f32;
            let color = Color {
                a: alpha,
                ..p.color
            };
            draw_cell_at(
                p.x - CELL * 0.5,
                p.y - CELL * 0.5,
                color,
                &self.cell_texture,
            );
        }
    }

    pub fn render_menu(&self, menu: &Menu, muted: bool) {
        clear_background(Color::from_rgba(10, 10, 18, 255));
        match menu.screen() {
            MenuScreen::Main => self.render_main_menu(menu),
            MenuScreen::HiScores => self.render_hi_scores(menu),
            MenuScreen::Controls => self.render_controls(),
        }
        let (label, color) = if muted {
            ("[M]  MUTED", Color::new(0.8, 0.4, 0.4, 1.0))
        } else {
            ("[M]  SOUND ON", GRAY)
        };
        self.draw_centered(label, screen_height() - 24.0, 14.0, color);
    }

    fn render_piece_preview(
        &self,
        kind: PieceKind,
        offsets: &[(i32, i32); 4],
        preview_y_offset: i32,
    ) {
        for &(dc, dr) in offsets {
            let c = 3 + dc;
            let r = -3 + dr + preview_y_offset;
            draw_cell(
                BOARD_X,
                BOARD_Y - PAD,
                c,
                r,
                piece_color(kind),
                &self.cell_texture,
            );
        }
    }

    fn render_board(&self, snapshot: &GameSnapshot) {
        let texture = &self.cell_texture;

        draw_rectangle(
            BOARD_X,
            BOARD_Y,
            BOARD_COLS as f32 * CELL,
            BOARD_ROWS as f32 * CELL,
            BOARD_BG,
        );

        // Ghost piece
        if let (Some(kind), Some(ghost_cells)) = (snapshot.active_kind, &snapshot.ghost_cells) {
            let base = piece_color(kind);
            let ghost_color = Color { a: 0.25, ..base };
            for &(c, r) in ghost_cells {
                if c >= 0 && r >= 0 && (r as usize) < BOARD_ROWS && (c as usize) < BOARD_COLS {
                    draw_cell(BOARD_X, BOARD_Y, c, r, ghost_color, texture);
                }
            }
        }

        // Locked cells (skip rows pending compaction — drawn as particles)
        for (r, row) in snapshot.board.iter().enumerate() {
            if snapshot.rows_pending_compaction.contains(&r) {
                continue;
            }
            for (c, cell) in row.iter().enumerate() {
                if let Some(kind) = cell {
                    let left_border = c == 0 || snapshot.board[r][c - 1].is_none();
                    let top_border = r == 0 || snapshot.board[r - 1][c].is_none();
                    let right_border = c == BOARD_COLS - 1 || snapshot.board[r][c + 1].is_none();
                    let bottom_border = r == BOARD_ROWS - 1 || snapshot.board[r + 1][c].is_none();
                    draw_cell_bordered(
                        BOARD_X,
                        BOARD_Y,
                        c as i32,
                        r as i32,
                        piece_color(*kind),
                        texture,
                        left_border,
                        top_border,
                        right_border,
                        bottom_border,
                    );
                }
            }
        }

        // Particles — now handled by the stateful particle system (rendered separately)

        // Active piece
        if let (Some(kind), Some(active_cells)) = (snapshot.active_kind, &snapshot.active_cells) {
            for &(c, r) in active_cells {
                if c >= 0 && r >= 0 && (r as usize) < BOARD_ROWS && (c as usize) < BOARD_COLS {
                    draw_cell(BOARD_X, BOARD_Y, c, r, piece_color(kind), texture);
                }
            }
        }

        self.render_piece_preview(
            snapshot.next_kind,
            &snapshot.next_preview_offsets,
            snapshot.next_preview_y_offset,
        );

        draw_rectangle(
            BOARD_X,
            BOARD_Y,
            BOARD_COLS as f32 * CELL,
            BOARD_ROWS as f32 * CELL,
            Color::new(0.0, 0.0, 0.0, 0.1),
        );
    }

    fn render_grade_bar(&self, snapshot: &GameSnapshot) {
        let score = snapshot.score;
        let grade = snapshot.grade;
        let (prev, next_opt) = Grade::grade_progress(score);
        let progress: f32 = match next_opt {
            None => 1.0,
            Some(next) => (score - prev) as f32 / (next - prev) as f32,
        };

        let bar_h = BOARD_ROWS as f32 * CELL;
        const SHADOW_PAD: f32 = 2.0;
        let inner_h = bar_h - SHADOW_PAD * 2.0;
        let fill_h = inner_h * progress;

        // Shadow flush with playfield vertically; extends SHADOW_PAD horizontally
        draw_rectangle(
            BAR_X - SHADOW_PAD,
            BOARD_Y,
            BAR_WIDTH + SHADOW_PAD * 2.0,
            bar_h,
            Color::new(0.0, 0.0, 0.0, 0.55),
        );

        draw_line(
            DIVIDER_X,
            BOARD_Y,
            DIVIDER_X,
            BOARD_Y + bar_h,
            1.5,
            Color::new(0.25, 0.25, 0.35, 1.0),
        );
        draw_rectangle(BAR_X, BOARD_Y + SHADOW_PAD, BAR_WIDTH, inner_h, BOARD_BG);
        draw_rectangle(
            BAR_X,
            BOARD_Y + SHADOW_PAD + inner_h - fill_h,
            BAR_WIDTH,
            fill_h,
            grade_bar_color(grade.index()),
        );
    }

    fn render_sidebar(&self, snapshot: &GameSnapshot) {
        const FONT_LG: f32 = 26.0;
        const FONT_SM: f32 = 18.0;
        const LH: f32 = 30.0;
        const DIM: Color = Color::new(0.5, 0.5, 0.5, 1.0);

        let x = SIDEBAR_X;
        let mut y = BOARD_Y + 22.0;

        self.draw_text("LEVEL", x, y, FONT_SM, DIM);
        y += LH;
        self.draw_text(&format!("{:03}", snapshot.level), x, y, FONT_LG, WHITE);
        y += 6.0;
        draw_line(x, y, x + 48.0, y, 2.0, DIM);
        y += 24.0;
        self.draw_text(
            &format!("{}", next_level_barrier(snapshot.level)),
            x,
            y,
            FONT_LG,
            WHITE,
        );
        y += LH + 8.0;

        self.draw_text("LINES", x, y, FONT_SM, DIM);
        y += LH;
        self.draw_text(&format!("{}", snapshot.lines), x, y, FONT_LG, WHITE);
        y += LH + 8.0;

        self.draw_text("TIME", x, y, FONT_SM, DIM);
        y += LH;
        self.draw_text(&format_time(snapshot.ticks_elapsed), x, y, FONT_LG, WHITE);
        y += LH + 8.0;

        self.draw_text("SCORE", x, y, FONT_SM, DIM);
        y += LH;
        self.draw_text(&format!("{}", snapshot.score), x, y, FONT_LG, WHITE);
        y += LH + 8.0;

        self.draw_text("GRADE", x, y, FONT_SM, DIM);
        y += LH;
        self.draw_text(&format!("{}", snapshot.grade), x, y, FONT_LG, WHITE);
        y += LH + 8.0;

        self.draw_text("NEXT", x, y, FONT_SM, DIM);
        y += LH;
        let (_, next_opt) = Grade::grade_progress(snapshot.score);
        let next_str = match next_opt {
            Some(n) => format!("{}", n),
            None => "??????".to_string(),
        };
        self.draw_text(&next_str, x, y, FONT_LG, WHITE);
    }

    pub fn render_ready(&self, snapshot: &GameSnapshot) {
        clear_background(grade_bg_color(snapshot.grade.index()));
        draw_rectangle(
            BOARD_X,
            BOARD_Y,
            BOARD_COLS as f32 * CELL,
            BOARD_ROWS as f32 * CELL,
            BOARD_BG,
        );
        self.render_piece_preview(
            snapshot.active_kind.unwrap_or(snapshot.next_kind),
            &snapshot.active_preview_offsets,
            snapshot.active_preview_y_offset,
        );
        draw_rectangle(
            BOARD_X,
            BOARD_Y,
            BOARD_COLS as f32 * CELL,
            BOARD_ROWS as f32 * CELL,
            Color::new(0.0, 0.0, 0.0, 0.1),
        );
        self.render_grade_bar(snapshot);
        self.render_sidebar(snapshot);
        let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
        let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;
        self.draw_centered_x("READY", cx, cy, 28.0, WHITE);
    }

    fn render_line_clear_overlay(&mut self, ticks_elapsed: u64) {
        // Clear expired overlay before extracting data.
        if matches!(&self.overlay, Some(o) if o.frames_remaining == 0) {
            self.overlay = None;
        }
        // Extract all data from overlay before any rendering calls to avoid borrow conflicts.
        let (label, opacity, hue_shift, frame_parity) = match &self.overlay {
            None => return,
            Some(o) => (
                o.label(),
                o.opacity(),
                o.hue_shift(ticks_elapsed),
                (o.frames_remaining % 2) as f32,
            ),
        };

        // Render text to off-screen target.
        set_camera(&Camera2D {
            zoom: vec2(2.0 / WINDOW_W, -2.0 / WINDOW_H),
            target: vec2(280.0, 390.0),
            render_target: Some(self.overlay_target.clone()),
            ..Default::default()
        });
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
        let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;
        self.draw_centered_x(label, cx, cy, 40.0, WHITE);
        set_default_camera();

        // Draw to screen with scanline shader.
        self.overlay_material
            .set_uniform("frame_parity", frame_parity);
        self.overlay_material.set_uniform("hue_shift", hue_shift);
        self.overlay_material
            .set_uniform("overlay_opacity", opacity);
        gl_use_material(&self.overlay_material);
        draw_texture_ex(
            &self.overlay_target.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(WINDOW_W, WINDOW_H)),
                flip_y: true,
                ..Default::default()
            },
        );
        gl_use_default_material();

        // Tick the overlay.
        let done = self
            .overlay
            .as_ref()
            .map_or(true, |o| o.frames_remaining == 0);
        if done {
            self.overlay = None;
        } else if let Some(o) = &mut self.overlay {
            o.frames_remaining -= 1;
        }
    }

    fn render_overlay(&self, snapshot: &GameSnapshot) {
        let cx = BOARD_X + BOARD_COLS as f32 * CELL * 0.5;
        let cy = BOARD_Y + BOARD_ROWS as f32 * CELL * 0.5;
        if snapshot.game_won {
            self.draw_text("LEVEL 999", cx - 60.0, cy - 16.0, 28.0, WHITE);
            self.draw_text(
                &format_time(snapshot.ticks_elapsed),
                cx - 50.0,
                cy + 20.0,
                22.0,
                LIGHTGRAY,
            );
        } else if snapshot.game_over {
            self.draw_text("GAME OVER", cx - 62.0, cy, 28.0, WHITE);
        }
    }

    fn render_main_menu(&self, menu: &Menu) {
        const FONT: f32 = 24.0;
        const LH: f32 = 36.0;

        let mode_str = match menu.game_mode() {
            GameMode::Master => "MASTER",
            GameMode::TwentyG => "20G",
        };
        let rot_str = match menu.rotation() {
            Kind::Ars => "ARS",
            Kind::Srs => "SRS",
        };

        let mode_label = maybe_bracket(mode_str, menu.cursor() == 0);
        let rot_label = maybe_bracket(rot_str, menu.cursor() == 1);
        let hi_label = maybe_bracket("HI SCORES", menu.cursor() == 2);
        let ctrl_label = maybe_bracket("CONTROLS", menu.cursor() == 3);
        let start_label = maybe_bracket("START", menu.cursor() == 4);

        let lines: &[Option<(&str, Color)>] = &[
            Some(("GAME MODE", GRAY)),
            Some((&mode_label, WHITE)),
            None,
            Some(("ROTATION", GRAY)),
            Some((&rot_label, WHITE)),
            None,
            Some((&hi_label, WHITE)),
            Some((&ctrl_label, WHITE)),
            None,
            Some((&start_label, WHITE)),
        ];

        let total_h = lines.len() as f32 * LH;
        let start_y = (screen_height() - total_h) / 2.0 + LH;

        for (i, line) in lines.iter().enumerate() {
            if let Some((text, color)) = line {
                self.draw_centered(text, start_y + i as f32 * LH, FONT, *color);
            }
        }
    }

    fn render_controls(&self) {
        const TITLE: f32 = 26.0;
        const HDR: f32 = 15.0;
        const ENTRY: f32 = 20.0;
        const HINT: f32 = 14.0;
        const LH: f32 = 32.0;

        let cx = screen_width() / 2.0;
        let cy = screen_height() / 2.0;

        self.draw_centered("CONTROLS", cy - LH * 5.0, TITLE, WHITE);

        let col_key = cx - 100.0;
        let col_action = cx + 100.0;

        let hdr_y = cy - LH * 3.5;
        self.draw_centered_x("KEY", col_key, hdr_y, HDR, GRAY);
        self.draw_centered_x("ACTION", col_action, hdr_y, HDR, GRAY);
        draw_line(
            cx - 200.0,
            hdr_y + 8.0,
            cx + 200.0,
            hdr_y + 8.0,
            1.0,
            DARKGRAY,
        );

        let rows: &[(&str, &str)] = &[
            ("Left / H", "Move left"),
            ("Right / L", "Move right"),
            ("Down / J", "Soft drop"),
            ("Space", "Sonic drop"),
            ("X", "Rotate CW"),
            ("Z", "Rotate CCW"),
            ("Backspace", "Back / quit"),
        ];

        for (i, (key, action)) in rows.iter().enumerate() {
            let y = cy - LH * 2.5 + i as f32 * LH;
            let color = LIGHTGRAY;
            self.draw_centered_x(key, col_key, y, ENTRY, color);
            self.draw_centered_x(action, col_action, y, ENTRY, color);
        }

        self.draw_centered("BKSP to go back", cy + LH * 4.5, HINT, GRAY);
    }

    fn render_hi_scores(&self, menu: &Menu) {
        const TITLE: f32 = 26.0;
        const HDR: f32 = 15.0;
        const ENTRY: f32 = 22.0;
        const HINT: f32 = 14.0;
        const LH: f32 = 36.0;

        let tab_names = ["MASTER / ARS", "MASTER / SRS", "20G / ARS", "20G / SRS"];
        let tab = menu.hi_scores_tab();
        let data = &menu.hi_scores_data()[tab];

        let cx = screen_width() / 2.0;
        let cy = screen_height() / 2.0;

        let label = format!("< {} >", tab_names[tab]);
        self.draw_centered(&label, cy - LH * 4.5, TITLE, WHITE);

        let col_rank = cx - 160.0;
        let col_grade = cx;
        let col_time = cx + 150.0;

        let hdr_y = cy - LH * 2.8;
        self.draw_centered_x("#", col_rank, hdr_y, HDR, GRAY);
        self.draw_centered_x("GRADE", col_grade, hdr_y, HDR, GRAY);
        self.draw_centered_x("TIME", col_time, hdr_y, HDR, GRAY);

        draw_line(
            cx - 200.0,
            hdr_y + 8.0,
            cx + 200.0,
            hdr_y + 8.0,
            1.0,
            DARKGRAY,
        );

        for i in 0..5usize {
            let y = cy - LH * 1.8 + i as f32 * LH;
            let color = if i == 0 { WHITE } else { LIGHTGRAY };
            self.draw_centered_x(&format!("{}", i + 1), col_rank, y, ENTRY, color);
            if let Some(entry) = data.get(i) {
                self.draw_centered_x(&format!("{}", entry.grade), col_grade, y, ENTRY, color);
                self.draw_centered_x(&format_time(entry.ticks), col_time, y, ENTRY, color);
            } else {
                self.draw_centered_x("---", col_grade, y, ENTRY, DARKGRAY);
                self.draw_centered_x("---", col_time, y, ENTRY, DARKGRAY);
            }
        }

        self.draw_centered("BKSP to go back", cy + LH * 3.5, HINT, GRAY);
    }
}

fn spawn_particles(
    particles: &mut Vec<Particle>,
    board: &crate::types::Board,
    rows: &[usize],
    count: u32,
) {
    use crate::constants::{PARTICLE_BASE_LIFETIME, PARTICLE_BASE_SPEED};

    let particles_per_cell: u32 = if count >= 4 { 3 } else { 1 };
    let speed_scale = match count {
        1 => 1.0,
        2 => 1.4,
        3 => 1.8,
        _ => 2.5,
    };

    for &r in rows {
        for (c, cell) in board[r].iter().enumerate() {
            if let Some(kind) = cell {
                for _ in 0..particles_per_cell {
                    // Base outward direction from horizontal center, slight upward bias.
                    let dist = c as f32 - (BOARD_COLS as f32 - 1.0) / 2.0;
                    let base_angle = dist.atan2(-1.5_f32); // negative y = upward in screen coords
                    let spread = (rand_f32() - 0.5) * std::f32::consts::FRAC_PI_3;
                    let angle = base_angle + spread;
                    let speed = PARTICLE_BASE_SPEED * speed_scale * (0.6 + 0.8 * rand_f32());

                    let lifetime = PARTICLE_BASE_LIFETIME + (rand_f32() * 25.0) as u32;
                    particles.push(Particle {
                        x: BOARD_X + c as f32 * CELL + CELL * 0.5,
                        y: BOARD_Y + r as f32 * CELL + CELL * 0.5,
                        vx: angle.sin() * speed,
                        vy: -angle.cos().abs() * speed,
                        age: 0,
                        lifetime,
                        color: piece_color(*kind),
                    });
                }
            }
        }
    }
}

fn make_cell_texture() -> Texture2D {
    const SIZE: usize = 32;
    let mut pixels = [255u8; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let fy = y as f32 / (SIZE - 1) as f32;

            let raw = if x == 0 || y == 0 {
                1.0
            } else {
                1.0 - 0.4 * fy
            };
            let quantized = (raw * 16.0).floor() / 16.0;
            let v = (quantized * 255.0) as u8;
            let i = (y * SIZE + x) * 4;
            pixels[i] = v;
            pixels[i + 1] = v;
            pixels[i + 2] = v;
            // alpha channel stays 255
        }
    }
    Texture2D::from_rgba8(SIZE as u16, SIZE as u16, &pixels)
}

/// Like draw_cell but draws white-grey border strips on the left and/or top edges when
/// the adjacent cell in that direction is unfilled.
fn draw_cell_bordered(
    origin_x: f32,
    origin_y: f32,
    col: i32,
    row: i32,
    color: Color,
    texture: &Texture2D,
    left_border: bool,
    top_border: bool,
    right_border: bool,
    bottom_border: bool,
) {
    const BORDER_COLOR: Color = Color::new(0.70, 0.70, 0.70, 1.0);
    let x = origin_x + col as f32 * CELL;
    let y = origin_y + row as f32 * CELL;
    if left_border {
        draw_rectangle(x, y, INSET, CELL, BORDER_COLOR);
    }
    if top_border {
        draw_rectangle(x, y, CELL, INSET, BORDER_COLOR);
    }
    if right_border {
        draw_rectangle(x + CELL - INSET, y, INSET, CELL, BORDER_COLOR);
    }
    if bottom_border {
        draw_rectangle(x, y + CELL - INSET, CELL, INSET, BORDER_COLOR);
    }
    draw_cell(origin_x, origin_y, col, row, color, texture);
}

/// Draw a single CELL×CELL block at pixel position (x, y).
fn draw_cell_at(x: f32, y: f32, color: Color, texture: &Texture2D) {
    draw_texture_ex(
        texture,
        x + INSET,
        y + INSET,
        color,
        DrawTextureParams {
            dest_size: Some(vec2(CELL - INSET * 2.0, CELL - INSET * 2.0)),
            ..Default::default()
        },
    );
}

/// Draw a single CELL×CELL block at grid position (col, row) relative to (origin_x, origin_y).
fn draw_cell(origin_x: f32, origin_y: f32, col: i32, row: i32, color: Color, texture: &Texture2D) {
    draw_cell_at(
        origin_x + col as f32 * CELL,
        origin_y + row as f32 * CELL,
        color,
        texture,
    );
}

fn grade_bar_color(grade_idx: usize) -> Color {
    match grade_idx % 7 {
        0 => Color::from_rgba(220, 50, 50, 200),
        1 => Color::from_rgba(230, 130, 0, 200),
        2 => Color::from_rgba(220, 210, 0, 200),
        3 => Color::from_rgba(50, 180, 50, 200),
        4 => Color::from_rgba(50, 100, 220, 200),
        5 => Color::from_rgba(80, 0, 200, 200),
        _ => Color::from_rgba(150, 0, 220, 200),
    }
}

fn grade_bg_color(grade_idx: usize) -> Color {
    let tint = grade_bar_color(grade_idx);
    Color::new(
        0.04 + tint.r * 0.14,
        0.04 + tint.g * 0.14,
        0.07 + tint.b * 0.14,
        1.0,
    )
}

fn piece_color(kind: PieceKind) -> Color {
    match kind {
        PieceKind::I => Color::from_rgba(200, 50, 50, 255),
        PieceKind::O => Color::from_rgba(220, 200, 0, 255),
        PieceKind::T => Color::from_rgba(0, 200, 200, 255),
        PieceKind::S => Color::from_rgba(200, 0, 200, 255),
        PieceKind::Z => Color::from_rgba(0, 160, 0, 255),
        PieceKind::J => Color::from_rgba(50, 100, 220, 255),
        PieceKind::L => Color::from_rgba(255, 150, 100, 255),
    }
}

pub fn format_time(ticks: u64) -> String {
    let seconds = ticks / 60;
    let ms = (ticks % 60) * 1000 / 60;
    let mm = seconds / 60;
    let ss = seconds % 60;
    format!("{:02}:{:02}.{:03}", mm, ss, ms)
}

fn next_level_barrier(level: u32) -> u32 {
    let round_up = (level + 1).next_multiple_of(100);
    if round_up == 1000 { 999 } else { round_up }
}

fn maybe_bracket(s: &str, active: bool) -> String {
    if active {
        format!("< {} >", s)
    } else {
        format!("  {}  ", s)
    }
}
