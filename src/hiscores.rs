use crate::judge::Grade;
use crate::menu::GameMode;
use crate::rotation_system::Kind;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct HiScoreEntry {
    pub grade: Grade,
    pub ticks: u64,
}

/// Insert `entry` into `entries` in sorted order (best first) and truncate to `max`.
/// Best = higher grade; ties broken by lower ticks.
pub fn insert_entry(entries: &mut Vec<HiScoreEntry>, entry: HiScoreEntry, max: usize) {
    entries.push(entry);
    // Sort best-first: higher grade wins; ties by lower ticks
    entries.sort_by(|a, b| b.grade.cmp(&a.grade).then(a.ticks.cmp(&b.ticks)));
    entries.truncate(max);
}

fn storage_key(mode: GameMode, rotation: Kind) -> &'static str {
    match (mode, rotation) {
        (GameMode::Master, Kind::Ars) => "hi_master_ars",
        (GameMode::Master, Kind::Srs) => "hi_master_srs",
        (GameMode::TwentyG, Kind::Ars) => "hi_20g_ars",
        (GameMode::TwentyG, Kind::Srs) => "hi_20g_srs",
    }
}

pub fn load(mode: GameMode, rotation: Kind) -> Vec<HiScoreEntry> {
    let storage = quad_storage::STORAGE.lock().unwrap();
    storage
        .get(storage_key(mode, rotation))
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(mode: GameMode, rotation: Kind, entries: Vec<HiScoreEntry>) {
    let mut storage = quad_storage::STORAGE.lock().unwrap();
    if let Ok(json) = serde_json::to_string(&entries) {
        storage.set(storage_key(mode, rotation), &json);
    }
}

pub fn submit(mode: GameMode, rotation: Kind, entry: HiScoreEntry) {
    let mut entries = load(mode, rotation);
    insert_entry(&mut entries, entry, 5);
    save(mode, rotation, entries);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judge::Grade;

    fn entry(grade: Grade, ticks: u64) -> HiScoreEntry {
        HiScoreEntry { grade, ticks }
    }

    #[test]
    fn higher_grade_ranks_first() {
        let mut entries = vec![entry(Grade::One, 1000)];
        insert_entry(&mut entries, entry(Grade::SOne, 2000), 5);
        assert!(matches!(entries[0].grade, Grade::SOne));
        assert!(matches!(entries[1].grade, Grade::One));
    }

    #[test]
    fn same_grade_lower_ticks_ranks_first() {
        let mut entries = vec![entry(Grade::STwo, 5000)];
        insert_entry(&mut entries, entry(Grade::STwo, 3000), 5);
        assert_eq!(entries[0].ticks, 3000);
        assert_eq!(entries[1].ticks, 5000);
    }

    #[test]
    fn truncates_to_max() {
        let mut entries: Vec<HiScoreEntry> =
            (0..5).map(|i| entry(Grade::One, 1000 + i * 100)).collect();
        // Worse than all existing: should not appear
        insert_entry(&mut entries, entry(Grade::Nine, 500), 5);
        assert_eq!(entries.len(), 5);
        assert!(entries.iter().all(|e| matches!(e.grade, Grade::One)));
    }

    #[test]
    fn better_entry_evicts_worst() {
        let mut entries: Vec<HiScoreEntry> =
            (0..5).map(|i| entry(Grade::One, 1000 + i * 100)).collect();
        insert_entry(&mut entries, entry(Grade::SOne, 999), 5);
        assert_eq!(entries.len(), 5);
        assert!(matches!(entries[0].grade, Grade::SOne));
    }
}
