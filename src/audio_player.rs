use crate::types::Grade;

pub trait AudioPlayer {
    fn piece_locked(&self);
    fn piece_begin_locking(&self);
    fn lines_cleared(&self, count: u32);
    fn ready(&self);
    fn grade_changed(&self, grade: Grade);
    fn game_over(&self);
    fn set_muted(&self, muted: bool);
    fn is_muted(&self) -> bool;
}

#[cfg(test)]
pub mod null {
    use super::{AudioPlayer, Grade};

    pub struct Null;

    impl AudioPlayer for Null {
        fn piece_locked(&self) {}
        fn piece_begin_locking(&self) {}
        fn lines_cleared(&self, _count: u32) {}
        fn ready(&self) {}
        fn grade_changed(&self, _grade: Grade) {}
        fn game_over(&self) {}
        fn set_muted(&self, _muted: bool) {}
        fn is_muted(&self) -> bool {
            false
        }
    }
}

pub mod macroquad {
    use super::{AudioPlayer, Grade};
    use ::macroquad::audio::{Sound, play_sound_once};
    use std::sync::atomic::{AtomicBool, Ordering};

    pub struct Macroquad {
        piece_locked: Sound,
        piece_begin_locking: Sound,
        ready: Sound,
        single: Sound,
        double: Sound,
        triple: Sound,
        fetris: Sound,
        game_over: Sound,
        // Indexed by grade ordinal: Nine=0, Eight=1, ..., SNine=17
        grades: Vec<Sound>,
        muted: AtomicBool,
    }

    impl Macroquad {
        pub async fn create() -> Self {
            let load = |path| ::macroquad::audio::load_sound(path);
            let grades = {
                const FILES: &[&str] = &[
                    "assets/audio/voice/grade_9.ogg",
                    "assets/audio/voice/grade_8.ogg",
                    "assets/audio/voice/grade_7.ogg",
                    "assets/audio/voice/grade_6.ogg",
                    "assets/audio/voice/grade_5.ogg",
                    "assets/audio/voice/grade_4.ogg",
                    "assets/audio/voice/grade_3.ogg",
                    "assets/audio/voice/grade_2.ogg",
                    "assets/audio/voice/grade_1.ogg",
                    "assets/audio/voice/grade_s1.ogg",
                    "assets/audio/voice/grade_s2.ogg",
                    "assets/audio/voice/grade_s3.ogg",
                    "assets/audio/voice/grade_s4.ogg",
                    "assets/audio/voice/grade_s5.ogg",
                    "assets/audio/voice/grade_s6.ogg",
                    "assets/audio/voice/grade_s7.ogg",
                    "assets/audio/voice/grade_s8.ogg",
                    "assets/audio/voice/grade_s9.ogg",
                ];
                let mut v = Vec::with_capacity(FILES.len());
                for &path in FILES {
                    v.push(load(path).await.unwrap());
                }
                v
            };
            Self {
                piece_locked: load("assets/audio/piece_locked.ogg").await.unwrap(),
                piece_begin_locking: load("assets/audio/piece_begin_locking.ogg").await.unwrap(),
                ready: load("assets/audio/voice/ready.ogg").await.unwrap(),
                single: load("assets/audio/voice/single.ogg").await.unwrap(),
                double: load("assets/audio/voice/double.ogg").await.unwrap(),
                triple: load("assets/audio/voice/triple.ogg").await.unwrap(),
                fetris: load("assets/audio/voice/fetris.ogg").await.unwrap(),
                game_over: load("assets/audio/voice/game_over.ogg").await.unwrap(),
                grades,
                muted: AtomicBool::new(false),
            }
        }

        fn play(&self, snd: &Sound) {
            if !self.muted.load(Ordering::Relaxed) {
                play_sound_once(snd);
            }
        }

        fn grade_sound(&self, grade: Grade) -> &Sound {
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
            &self.grades[idx]
        }
    }

    impl AudioPlayer for Macroquad {
        fn piece_locked(&self) {
            self.play(&self.piece_locked);
        }
        fn piece_begin_locking(&self) {
            self.play(&self.piece_begin_locking);
        }
        fn lines_cleared(&self, count: u32) {
            let snd = match count {
                1 => &self.single,
                2 => &self.double,
                3 => &self.triple,
                _ => &self.fetris,
            };
            self.play(snd);
        }
        fn ready(&self) {
            self.play(&self.ready);
        }
        fn grade_changed(&self, grade: Grade) {
            self.play(self.grade_sound(grade));
        }
        fn game_over(&self) {
            self.play(&self.game_over);
        }
        fn set_muted(&self, muted: bool) {
            self.muted.store(muted, Ordering::Relaxed);
        }
        fn is_muted(&self) -> bool {
            self.muted.load(Ordering::Relaxed)
        }
    }
}
