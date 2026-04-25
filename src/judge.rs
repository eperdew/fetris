use bevy::prelude::*;
use crate::data::{Grade, HiScoreEntry, JudgeEvent};

#[derive(Resource)]
pub struct Judge {
    combo: u32,
    score: u32,
    best_grade: Grade,
    grade_ticks: u64,
}

impl Judge {
    pub fn new() -> Self {
        Self { combo: 1, score: 0, best_grade: Grade::Nine, grade_ticks: 0 }
    }

    pub fn on_event(&mut self, event: &JudgeEvent) {
        match *event {
            JudgeEvent::LockedWithoutClear => self.combo = 1,
            JudgeEvent::ClearedLines { level, cleared_playfield, num_lines,
                frames_soft_drop_held, sonic_drop_rows, ticks_elapsed } => {
                self.combo += 2 * num_lines - 2;
                let bravo = if cleared_playfield { 4 } else { 1 };
                self.score += ((level + 3) / 4 + frames_soft_drop_held + 2 * sonic_drop_rows)
                    * num_lines * self.combo * bravo;
                let new_grade = Grade::of_score(self.score);
                if new_grade > self.best_grade {
                    self.best_grade = new_grade;
                    self.grade_ticks = ticks_elapsed;
                }
            }
        }
    }

    pub fn score(&self) -> u32 { self.score }
    pub fn grade(&self) -> Grade { Grade::of_score(self.score) }
    pub fn grade_entry(&self) -> HiScoreEntry {
        HiScoreEntry { grade: self.best_grade, ticks: self.grade_ticks }
    }
}

impl Default for Judge {
    fn default() -> Self { Self::new() }
}

/// Bevy system: drains JudgeEvents and feeds them into the Judge resource.
pub fn judge_system(
    mut judge: ResMut<Judge>,
    mut events: MessageReader<JudgeEvent>,
) {
    for event in events.read() {
        judge.on_event(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear_event(level: u32, num_lines: u32, ticks_elapsed: u64) -> JudgeEvent {
        JudgeEvent::ClearedLines {
            level, cleared_playfield: false, num_lines,
            frames_soft_drop_held: 0, sonic_drop_rows: 0, ticks_elapsed,
        }
    }

    #[test]
    fn grade_entry_records_first_crossing() {
        let mut j = Judge::new();
        j.on_event(&clear_event(100, 4, 1000));
        let entry = j.grade_entry();
        assert!(entry.grade > Grade::Nine);
        assert_eq!(entry.ticks, 1000);
    }

    #[test]
    fn grade_entry_ticks_not_updated_on_same_grade() {
        let mut j = Judge::new();
        j.on_event(&clear_event(100, 4, 500));
        let g1 = j.grade_entry().grade;
        j.on_event(&JudgeEvent::LockedWithoutClear);
        j.on_event(&clear_event(100, 1, 999));
        let entry = j.grade_entry();
        assert_eq!(entry.grade, g1);
        assert_eq!(entry.ticks, 500);
    }

    #[test]
    fn grade_entry_initial_state() {
        let j = Judge::new();
        let entry = j.grade_entry();
        assert!(matches!(entry.grade, Grade::Nine));
        assert_eq!(entry.ticks, 0);
    }
}
