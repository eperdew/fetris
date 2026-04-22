pub trait AudioPlayer {
    fn piece_locked(&self);
    fn piece_begin_locking(&self);
    fn lines_cleared(&self, count: u32);
    fn ready(&self);
}

pub mod null {
    use super::AudioPlayer;

    pub struct Null;

    impl AudioPlayer for Null {
        fn piece_locked(&self) {}
        fn piece_begin_locking(&self) {}
        fn lines_cleared(&self, _count: u32) {}
        fn ready(&self) {}
    }
}

pub mod macroquad {
    use super::AudioPlayer;
    use ::macroquad::audio::{Sound, play_sound_once};

    pub struct Macroquad {
        pub piece_locked: Sound,
        pub piece_begin_locking: Sound,
        pub line_clear: Sound,
        pub ready: Sound,
    }

    impl Macroquad {
        pub async fn create() -> Self {
            let load = |path| ::macroquad::audio::load_sound(path);
            Self {
                piece_locked: load("assets/audio/piece_locked.ogg").await.unwrap(),
                piece_begin_locking: load("assets/audio/piece_begin_locking.ogg").await.unwrap(),
                line_clear: load("assets/audio/line_clear.ogg").await.unwrap(),
                ready: load("assets/audio/ready.ogg").await.unwrap(),
            }
        }
    }

    impl AudioPlayer for Macroquad {
        fn piece_locked(&self) {
            play_sound_once(&self.piece_locked);
        }
        fn piece_begin_locking(&self) {
            play_sound_once(&self.piece_begin_locking);
        }
        fn lines_cleared(&self, _count: u32) {
            play_sound_once(&self.line_clear);
        }
        fn ready(&self) {
            play_sound_once(&self.ready);
        }
    }
}
