use std::fmt;
pub struct Judge {
    combo: u32,
    score: u32,
}

pub enum JudgeEvent {
    LockedWithoutClear,
    ClearedLines {
        level: u32,
        cleared_playfield: bool,
        num_lines: u32,
    },
}

#[derive(Clone, Copy)]
pub enum Grade {
    Nine,
    Eight,
    Seven,
    Six,
    Five,
    Four,
    Three,
    Two,
    One,
    SOne,
    STwo,
    SThree,
    SFour,
    SFive,
    SSix,
    SSeven,
    SEight,
    SNine,
}

impl Grade {
    const SCORE_TABLE: &[(u32, Grade)] = &[
        (0, Grade::Nine),
        (400, Grade::Eight),
        (800, Grade::Seven),
        (1400, Grade::Six),
        (2000, Grade::Five),
        (3500, Grade::Four),
        (5500, Grade::Three),
        (8000, Grade::Two),
        (12000, Grade::One),
        (16000, Grade::SOne),
        (22000, Grade::STwo),
        (30000, Grade::SThree),
        (40000, Grade::SFour),
        (52000, Grade::SFive),
        (66000, Grade::SSix),
        (82000, Grade::SSeven),
        (100000, Grade::SEight),
        (120000, Grade::SNine),
    ];

    pub fn of_score(score: u32) -> Self {
        Self::SCORE_TABLE
            .iter()
            .rev()
            .find(|(threshold, _)| score >= *threshold)
            .map(|(_, g)| *g)
            .unwrap_or(Grade::Nine)
    }
}

impl fmt::Display for Grade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let as_string = match self {
            Self::Nine => "9",
            Self::Eight => "8",
            Self::Seven => "7",
            Self::Six => "6",
            Self::Five => "5",
            Self::Four => "4",
            Self::Three => "3",
            Self::Two => "2",
            Self::One => "1",
            Self::SOne => "S1",
            Self::STwo => "S2",
            Self::SThree => "S3",
            Self::SFour => "S4",
            Self::SFive => "S5",
            Self::SSix => "S6",
            Self::SSeven => "S7",
            Self::SEight => "S8",
            Self::SNine => "S9",
        };
        write!(f, "{:>2}", as_string)
    }
}

// TODO: Add tests for judge specifically.
impl Judge {
    pub fn on_event(&mut self, event: &JudgeEvent) {
        match *event {
            JudgeEvent::LockedWithoutClear => self.combo = 0,
            JudgeEvent::ClearedLines {
                level,
                cleared_playfield,
                num_lines,
            } => {
                self.combo += 2 * num_lines - 2;
                let bravo = if cleared_playfield { 4 } else { 1 };
                self.score += (level + 3) / 4 * num_lines * self.combo * bravo;
            }
        }
    }

    pub fn score(&self) -> u32 {
        self.score
    }

    pub fn grade(&self) -> Grade {
        Grade::of_score(self.score)
    }

    pub fn new() -> Self {
        Self { combo: 0, score: 0 }
    }
}
