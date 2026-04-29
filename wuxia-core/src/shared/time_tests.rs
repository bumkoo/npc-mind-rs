use super::*;

// =======================================================================
// Watch tests [v1.1]
// =======================================================================

#[test]
fn watch_index_roundtrip() {
    for w in Watch::all() {
        assert_eq!(Watch::from_index(w.index()), *w);
    }
}

#[test]
fn watch_next_cycle() {
    assert_eq!(Watch::Dawn.next(), Watch::Morning);
    assert_eq!(Watch::Morning.next(), Watch::Midday);
    assert_eq!(Watch::Midday.next(), Watch::Afternoon);
    assert_eq!(Watch::Afternoon.next(), Watch::Evening);
    assert_eq!(Watch::Evening.next(), Watch::Night);
    assert_eq!(Watch::Night.next(), Watch::Dawn); // wraps
}

#[test]
fn watch_is_last() {
    assert!(!Watch::Dawn.is_last());
    assert!(!Watch::Midday.is_last());
    assert!(Watch::Night.is_last());
}

#[test]
fn watch_default_is_dawn() {
    assert_eq!(Watch::default(), Watch::Dawn);
}

#[test]
fn watch_ordering() {
    assert!(Watch::Dawn < Watch::Morning);
    assert!(Watch::Morning < Watch::Midday);
    assert!(Watch::Evening < Watch::Night);
}

#[test]
fn watch_watches_until() {
    // Same direction
    assert_eq!(Watch::Dawn.watches_until(Watch::Morning), 1);
    assert_eq!(Watch::Dawn.watches_until(Watch::Night), 5);
    // Wrap around
    assert_eq!(Watch::Night.watches_until(Watch::Dawn), 1);
    assert_eq!(Watch::Afternoon.watches_until(Watch::Morning), 4);
    // Full cycle (same watch = 6, not 0)
    assert_eq!(Watch::Dawn.watches_until(Watch::Dawn), 6);
    assert_eq!(Watch::Night.watches_until(Watch::Night), 6);
}

#[test]
fn watch_display() {
    assert_eq!(Watch::Dawn.to_string(), "Dawn");
    assert_eq!(Watch::Night.to_string(), "Night");
}

#[test]
fn watch_translatable() {
    assert_eq!(Watch::Dawn.translation_key(), "watch.dawn");
    assert_eq!(Watch::Night.translation_key(), "watch.night");
}

#[test]
fn watch_serialization_roundtrip() {
    for w in Watch::all() {
        let json = serde_json::to_string(w).unwrap();
        let restored: Watch = serde_json::from_str(&json).unwrap();
        assert_eq!(*w, restored);
    }
}

#[test]
fn watch_all_returns_six() {
    assert_eq!(Watch::all().len(), 6);
    assert_eq!(Watch::all()[0], Watch::Dawn);
    assert_eq!(Watch::all()[5], Watch::Night);
}

// =======================================================================
// GameTime — Construction (backward compatible)
// =======================================================================

#[test]
fn create_valid_game_time() {
    let t = GameTime::new(1200, 3, 15);
    assert_eq!(t.year(), 1200);
    assert_eq!(t.month(), 3);
    assert_eq!(t.day(), 15);
    assert_eq!(t.watch(), Watch::Dawn); // default
}

#[test]
fn create_with_watch() {
    let t = GameTime::with_watch(1200, 3, 15, Watch::Midday);
    assert_eq!(t.year(), 1200);
    assert_eq!(t.day(), 15);
    assert_eq!(t.watch(), Watch::Midday);
}

#[test]
fn try_new_valid() {
    let t = GameTime::try_new(1200, 12, 30);
    assert!(t.is_some());
    assert_eq!(t.unwrap().watch(), Watch::Dawn);
}

#[test]
fn try_new_with_watch_valid() {
    let t = GameTime::try_new_with_watch(1200, 6, 15, Watch::Evening);
    assert!(t.is_some());
    assert_eq!(t.unwrap().watch(), Watch::Evening);
}

#[test]
fn try_new_invalid_month() {
    assert!(GameTime::try_new(1200, 0, 15).is_none());
    assert!(GameTime::try_new(1200, 13, 15).is_none());
}

#[test]
fn try_new_invalid_day() {
    assert!(GameTime::try_new(1200, 1, 0).is_none());
    assert!(GameTime::try_new(1200, 1, 31).is_none());
}

#[test]
#[should_panic(expected = "invalid date")]
fn new_panics_on_invalid_month() {
    GameTime::new(1200, 13, 1);
}

#[test]
#[should_panic(expected = "invalid date")]
fn new_panics_on_invalid_day() {
    GameTime::new(1200, 1, 31);
}

// =======================================================================
// Season (unchanged)
// =======================================================================

#[test]
fn season_from_month() {
    // Winter: 12, 1, 2
    assert_eq!(Season::from_month(12), Season::Winter);
    assert_eq!(Season::from_month(1), Season::Winter);
    assert_eq!(Season::from_month(2), Season::Winter);
    // Spring: 3, 4, 5
    assert_eq!(Season::from_month(3), Season::Spring);
    assert_eq!(Season::from_month(5), Season::Spring);
    // Summer: 6, 7, 8
    assert_eq!(Season::from_month(6), Season::Summer);
    assert_eq!(Season::from_month(8), Season::Summer);
    // Autumn: 9, 10, 11
    assert_eq!(Season::from_month(9), Season::Autumn);
    assert_eq!(Season::from_month(11), Season::Autumn);
}

#[test]
fn game_time_season() {
    assert_eq!(GameTime::new(1200, 1, 1).season(), Season::Winter);
    assert_eq!(GameTime::new(1200, 4, 15).season(), Season::Spring);
    assert_eq!(GameTime::new(1200, 7, 30).season(), Season::Summer);
    assert_eq!(GameTime::new(1200, 10, 1).season(), Season::Autumn);
}

// =======================================================================
// Day-level arithmetic (backward compatible)
// =======================================================================

#[test]
fn next_day_within_month() {
    let today = GameTime::new(1200, 3, 15);
    let tomorrow = today.next_day();
    assert_eq!(tomorrow, GameTime::new(1200, 3, 16));
}

#[test]
fn next_day_month_boundary() {
    let last_day = GameTime::new(1200, 3, 30);
    let next = last_day.next_day();
    assert_eq!(next, GameTime::new(1200, 4, 1));
}

#[test]
fn next_day_year_boundary() {
    let year_end = GameTime::new(1200, 12, 30);
    let new_year = year_end.next_day();
    assert_eq!(new_year, GameTime::new(1201, 1, 1));
}

#[test]
fn next_day_resets_to_dawn() {
    let midday = GameTime::with_watch(1200, 3, 15, Watch::Midday);
    let tomorrow = midday.next_day();
    assert_eq!(tomorrow.watch(), Watch::Dawn);
}

#[test]
fn days_between_same_day() {
    let t = GameTime::new(1200, 6, 15);
    assert_eq!(t.days_between(&t), 0);
}

#[test]
fn days_between_forward() {
    let a = GameTime::new(1200, 1, 1);
    let b = GameTime::new(1200, 1, 11);
    assert_eq!(a.days_between(&b), 10);
}

#[test]
fn days_between_backward() {
    let a = GameTime::new(1200, 1, 11);
    let b = GameTime::new(1200, 1, 1);
    assert_eq!(a.days_between(&b), -10);
}

#[test]
fn days_between_across_years() {
    let a = GameTime::new(1200, 1, 1);
    let b = GameTime::new(1201, 1, 1);
    assert_eq!(a.days_between(&b), DAYS_PER_YEAR as i64);
}

#[test]
fn advance_days() {
    let start = GameTime::new(1200, 1, 1);
    let result = start.advance_days(45);
    assert_eq!(result, GameTime::new(1200, 2, 16));
}

#[test]
fn advance_full_year() {
    let start = GameTime::new(1200, 1, 1);
    let result = start.advance_days(DAYS_PER_YEAR);
    assert_eq!(result, GameTime::new(1201, 1, 1));
}

// =======================================================================
// Watch-level arithmetic [v1.1]
// =======================================================================

#[test]
fn next_watch_within_day() {
    let dawn = GameTime::new(1200, 3, 15);
    let morning = dawn.next_watch();
    assert_eq!(morning, GameTime::with_watch(1200, 3, 15, Watch::Morning));
}

#[test]
fn next_watch_night_to_dawn_advances_day() {
    let night = GameTime::with_watch(1200, 3, 15, Watch::Night);
    let next_dawn = night.next_watch();
    assert_eq!(next_dawn, GameTime::new(1200, 3, 16));
}

#[test]
fn next_watch_year_boundary() {
    let year_end_night = GameTime::with_watch(1200, 12, 30, Watch::Night);
    let new_year_dawn = year_end_night.next_watch();
    assert_eq!(new_year_dawn, GameTime::new(1201, 1, 1));
}

#[test]
fn total_watches_roundtrip() {
    let dates = vec![
        GameTime::new(0, 1, 1),
        GameTime::with_watch(0, 1, 1, Watch::Night),
        GameTime::new(1200, 1, 1),
        GameTime::with_watch(1200, 6, 15, Watch::Midday),
        GameTime::with_watch(1200, 12, 30, Watch::Night),
        GameTime::new(1201, 1, 1),
    ];

    for date in dates {
        let total = date.to_total_watches();
        let restored = GameTime::from_total_watches(total);
        assert_eq!(date, restored, "Roundtrip failed for {:?}", date);
    }
}

#[test]
fn total_watches_day_boundary() {
    let dawn = GameTime::new(1200, 1, 1);
    let night = GameTime::with_watch(1200, 1, 1, Watch::Night);
    assert_eq!(night.to_total_watches() - dawn.to_total_watches(), 5);

    let next_dawn = GameTime::new(1200, 1, 2);
    assert_eq!(next_dawn.to_total_watches() - night.to_total_watches(), 1);
}

#[test]
fn six_next_watches_equal_one_day() {
    let start = GameTime::new(1200, 3, 15);
    let mut current = start;
    for _ in 0..6 {
        current = current.next_watch();
    }
    assert_eq!(current, GameTime::new(1200, 3, 16));
}

#[test]
fn watches_in_a_year() {
    let start = GameTime::new(1200, 1, 1);
    let end = GameTime::new(1201, 1, 1);
    assert_eq!(
        end.to_total_watches() - start.to_total_watches(),
        WATCHES_PER_YEAR as u64
    );
}

#[test]
fn watches_between() {
    let a = GameTime::new(1200, 1, 1);
    let b = GameTime::with_watch(1200, 1, 1, Watch::Midday);
    assert_eq!(a.watches_between(&b), 2);

    let c = GameTime::new(1200, 1, 2);
    assert_eq!(a.watches_between(&c), 6);
}

// =======================================================================
// Ordering (now includes watch)
// =======================================================================

#[test]
fn ordering() {
    let earlier = GameTime::new(1200, 3, 15);
    let later = GameTime::new(1200, 3, 16);
    assert!(earlier < later);
    assert!(later > earlier);
}

#[test]
fn ordering_across_months() {
    let march = GameTime::new(1200, 3, 30);
    let april = GameTime::new(1200, 4, 1);
    assert!(march < april);
}

#[test]
fn ordering_same_day_different_watch() {
    let dawn = GameTime::new(1200, 3, 15);
    let midday = GameTime::with_watch(1200, 3, 15, Watch::Midday);
    let night = GameTime::with_watch(1200, 3, 15, Watch::Night);
    assert!(dawn < midday);
    assert!(midday < night);
}

#[test]
fn equality_requires_same_watch() {
    let dawn = GameTime::new(1200, 3, 15);
    let also_dawn = GameTime::with_watch(1200, 3, 15, Watch::Dawn);
    let midday = GameTime::with_watch(1200, 3, 15, Watch::Midday);
    assert_eq!(dawn, also_dawn);
    assert_ne!(dawn, midday);
}

// =======================================================================
// Boundary checks
// =======================================================================

#[test]
fn is_last_day_of_year() {
    assert!(GameTime::new(1200, 12, 30).is_last_day_of_year());
    assert!(!GameTime::new(1200, 12, 29).is_last_day_of_year());
    assert!(!GameTime::new(1200, 11, 30).is_last_day_of_year());
}

#[test]
fn is_last_day_of_month() {
    assert!(GameTime::new(1200, 6, 30).is_last_day_of_month());
    assert!(!GameTime::new(1200, 6, 29).is_last_day_of_month());
}

// =======================================================================
// Display [v1.1 updated]
// =======================================================================

#[test]
fn display_format() {
    let t = GameTime::new(1200, 3, 15);
    assert_eq!(t.to_string(), "Y1200-M03-D15 Dawn (Spring)");
}

#[test]
fn display_format_with_watch() {
    let t = GameTime::with_watch(1200, 10, 1, Watch::Night);
    assert_eq!(t.to_string(), "Y1200-M10-D01 Night (Autumn)");
}

// =======================================================================
// Roundtrip: total_days (backward compatible)
// =======================================================================

#[test]
fn total_days_roundtrip() {
    let dates = vec![
        GameTime::new(0, 1, 1),
        GameTime::new(1200, 1, 1),
        GameTime::new(1200, 6, 15),
        GameTime::new(1200, 12, 30),
        GameTime::new(1201, 1, 1),
        GameTime::new(9999, 12, 30),
    ];

    for date in dates {
        let total = date.to_total_days();
        let restored = GameTime::from_total_days(total);
        assert_eq!(date, restored, "Roundtrip failed for {:?}", date);
    }
}

// =======================================================================
// Serialization
// =======================================================================

#[test]
fn serialization_roundtrip() {
    let original = GameTime::new(1200, 7, 22);
    let json = serde_json::to_string(&original).unwrap();
    let restored: GameTime = serde_json::from_str(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn serialization_with_watch() {
    let original = GameTime::with_watch(1200, 7, 22, Watch::Evening);
    let json = serde_json::to_string(&original).unwrap();
    let restored: GameTime = serde_json::from_str(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn deserialization_without_watch_defaults_to_dawn() {
    // Simulate old format without watch field
    let json = r#"{"year":1200,"month":3,"day":15}"#;
    let restored: GameTime = serde_json::from_str(json).unwrap();
    assert_eq!(restored.watch(), Watch::Dawn);
    assert_eq!(restored, GameTime::new(1200, 3, 15));
}

// =======================================================================
// Constants
// =======================================================================

#[test]
fn constants_consistent() {
    assert_eq!(DAYS_PER_YEAR, 360);
    assert_eq!(WATCHES_PER_DAY, 6);
    assert_eq!(WATCHES_PER_YEAR, 2160);
    assert_eq!(WATCHES_PER_YEAR, DAYS_PER_YEAR * WATCHES_PER_DAY);
}
