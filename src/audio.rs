use bevy::audio::{AudioPlayer, PlaybackSettings};
use bevy::prelude::*;
use bevy_pkv::PkvStore;

use crate::data::{GameEvent, Grade};

#[derive(Resource)]
pub struct AudioHandles {
    pub piece_begin_locking: Handle<AudioSource>,
    pub ready: Handle<AudioSource>,
    pub single: Handle<AudioSource>,
    pub double: Handle<AudioSource>,
    pub triple: Handle<AudioSource>,
    pub fetris: Handle<AudioSource>,
    pub game_over: Handle<AudioSource>,
    // Index 0 = Grade::Nine (worst), 17 = Grade::SNine (best)
    pub grades: Vec<Handle<AudioSource>>,
}

pub fn setup_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
    let grade_files = [
        "audio/grade_9.ogg",
        "audio/grade_8.ogg",
        "audio/grade_7.ogg",
        "audio/grade_6.ogg",
        "audio/grade_5.ogg",
        "audio/grade_4.ogg",
        "audio/grade_3.ogg",
        "audio/grade_2.ogg",
        "audio/grade_1.ogg",
        "audio/grade_s1.ogg",
        "audio/grade_s2.ogg",
        "audio/grade_s3.ogg",
        "audio/grade_s4.ogg",
        "audio/grade_s5.ogg",
        "audio/grade_s6.ogg",
        "audio/grade_s7.ogg",
        "audio/grade_s8.ogg",
        "audio/grade_s9.ogg",
    ];
    commands.insert_resource(AudioHandles {
        piece_begin_locking: asset_server.load("audio/piece_begin_locking.wav"),
        ready: asset_server.load("audio/ready.ogg"),
        single: asset_server.load("audio/single.ogg"),
        double: asset_server.load("audio/double.ogg"),
        triple: asset_server.load("audio/triple.ogg"),
        fetris: asset_server.load("audio/fetris.ogg"),
        game_over: asset_server.load("audio/game_over.ogg"),
        grades: grade_files
            .iter()
            .map(|f| asset_server.load(*f))
            .collect(),
    });
}

fn grade_handle(handles: &AudioHandles, grade: Grade) -> Handle<AudioSource> {
    let idx = match grade {
        Grade::Nine => 0,
        Grade::Eight => 1,
        Grade::Seven => 2,
        Grade::Six => 3,
        Grade::Five => 4,
        Grade::Four => 5,
        Grade::Three => 6,
        Grade::Two => 7,
        Grade::One => 8,
        Grade::SOne => 9,
        Grade::STwo => 10,
        Grade::SThree => 11,
        Grade::SFour => 12,
        Grade::SFive => 13,
        Grade::SSix => 14,
        Grade::SSeven => 15,
        Grade::SEight => 16,
        Grade::SNine => 17,
    };
    handles.grades[idx].clone()
}

pub fn audio_event_system(
    mut commands: Commands,
    mut events: MessageReader<GameEvent>,
    handles: Res<AudioHandles>,
    pkv: Res<PkvStore>,
) {
    if pkv.get::<bool>("muted").unwrap_or(false) {
        return;
    }
    for event in events.read() {
        let handle: Handle<AudioSource> = match event {
            GameEvent::PieceBeganLocking => handles.piece_begin_locking.clone(),
            GameEvent::LineClear { count } => match count {
                1 => handles.single.clone(),
                2 => handles.double.clone(),
                3 => handles.triple.clone(),
                _ => handles.fetris.clone(),
            },
            GameEvent::GradeAdvanced(grade) => grade_handle(&handles, *grade),
            GameEvent::GameEnded => handles.game_over.clone(),
        };
        commands.spawn((AudioPlayer::new(handle), PlaybackSettings::DESPAWN));
    }
}

pub fn play_ready_sound(
    mut commands: Commands,
    handles: Res<AudioHandles>,
    pkv: Res<PkvStore>,
) {
    if !pkv.get::<bool>("muted").unwrap_or(false) {
        commands.spawn((AudioPlayer::new(handles.ready.clone()), PlaybackSettings::DESPAWN));
    }
}
