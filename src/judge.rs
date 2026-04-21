use crate::types::{Grade, HiScoreEntry, JudgeEvent};

pub(crate) struct Judge {
    combo: u32,
    score: u32,
    best_grade: Grade,
    grade_ticks: u64,
}

impl Judge {
    // See https://tetris.wiki/Tetris_The_Grand_Master#Scoring_formula
    pub fn on_event(&mut self, event: &JudgeEvent) {
        match *event {
            JudgeEvent::LockedWithoutClear => self.combo = 1,
            JudgeEvent::ClearedLines {
                level,
                cleared_playfield,
                num_lines,
                frames_soft_drop_held,
                sonic_drop_rows,
                ticks_elapsed,
            } => {
                self.combo += 2 * num_lines - 2;
                let bravo = if cleared_playfield { 4 } else { 1 };
                self.score += ((level + 3) / 4 + frames_soft_drop_held + 2 * sonic_drop_rows)
                    * num_lines
                    * self.combo
                    * bravo;
                let new_grade = Grade::of_score(self.score);
                if new_grade > self.best_grade {
                    self.best_grade = new_grade;
                    self.grade_ticks = ticks_elapsed;
                }
            }
        }
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn grade(&self) -> Grade {
        Grade::of_score(self.score)
    }

    pub fn grade_entry(&self) -> HiScoreEntry {
        HiScoreEntry {
            grade: self.best_grade,
            ticks: self.grade_ticks,
        }
    }

    pub fn new() -> Self {
        Self {
            combo: 1,
            score: 0,
            best_grade: Grade::Nine,
            grade_ticks: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear_event(level: u32, num_lines: u32, ticks_elapsed: u64) -> JudgeEvent {
        JudgeEvent::ClearedLines {
            level,
            cleared_playfield: false,
            num_lines,
            frames_soft_drop_held: 0,
            sonic_drop_rows: 0,
            ticks_elapsed,
        }
    }

    #[test]
    fn grade_entry_records_first_crossing() {
        let mut j = Judge::new();
        // Score needs to reach 400 for Grade::Eight. Level 100 clears give
        // ((100+3)/4) * lines * combo * 1 = 25 * 1 * 1 = 25 per single-line clear.
        // Clearing 4 lines at once: 25 * 4 * (1 + 2*4-2) = 25 * 4 * 7 = 700 → Grade::Seven.
        j.on_event(&clear_event(100, 4, 1000));
        let entry = j.grade_entry();
        assert!(entry.grade > Grade::Nine, "should have improved from Nine");
        assert_eq!(entry.ticks, 1000, "ticks should be from the first crossing");
    }

    #[test]
    fn grade_entry_ticks_not_updated_on_same_grade() {
        let mut j = Judge::new();
        // First clear: cross into a new grade at tick 500.
        j.on_event(&clear_event(100, 4, 500));
        let grade_after_first = j.grade_entry().grade;
        // Second clear at a higher tick that doesn't advance the grade.
        j.on_event(&JudgeEvent::LockedWithoutClear);
        j.on_event(&clear_event(100, 1, 999));
        let entry = j.grade_entry();
        assert_eq!(entry.grade, grade_after_first, "grade should be unchanged");
        assert_eq!(
            entry.ticks, 500,
            "ticks should still reflect the first crossing"
        );
    }

    #[test]
    fn grade_entry_initial_state() {
        let j = Judge::new();
        let entry = j.grade_entry();
        assert!(matches!(entry.grade, Grade::Nine));
        assert_eq!(entry.ticks, 0);
    }
}
