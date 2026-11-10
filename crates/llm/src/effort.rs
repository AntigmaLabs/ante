//! Effort ladder resolution shared by provider profiles. Every resolver here
//! rounds a requested effort down to the nearest level at or below it, and up
//! to the lowest level when every level is above it.

use ante_protocol_shape::Effort;

/// The setting of the rung `requested` rounds to. `rungs` must be non-empty
/// and ascend by level.
pub fn resolve<T: Copy>(rungs: &[(Effort, T)], requested: Effort) -> T {
    debug_assert!(rungs.windows(2).all(|w| w[0].0 < w[1].0), "ladder rungs must ascend");
    round_down(rungs.iter().copied(), requested).expect("ladder rungs must be non-empty")
}

/// The wire setting for `requested` on a model that takes `levels`: the
/// setting of the rung it rounds to among those of `spelling` (every level the
/// wire can express, ascending) whose level the model takes. `None` when the
/// model takes no level the wire can spell.
pub fn resolve_among<T: Copy>(
    spelling: &[(Effort, T)],
    levels: &[Effort],
    requested: Effort,
) -> Option<T> {
    round_down(spelling.iter().copied().filter(|(level, _)| levels.contains(level)), requested)
}

/// The levels a model that takes `levels` offers on a wire: those the wire
/// spells, ascending. `spelling` is as for [`resolve_among`].
pub fn offered_levels<T>(spelling: &[(Effort, T)], levels: &[Effort]) -> Vec<Effort> {
    spelling.iter().map(|(level, _)| *level).filter(|level| levels.contains(level)).collect()
}

/// The one rounding rule: the setting of the highest rung at or below
/// `requested`, or the lowest rung's when every rung is above it. `None` only
/// for an empty ladder.
fn round_down<T>(rungs: impl IntoIterator<Item = (Effort, T)>, requested: Effort) -> Option<T> {
    let mut selected = None;
    for (level, setting) in rungs {
        if selected.is_some() && level > requested {
            break;
        }
        selected = Some(setting);
    }
    selected
}

/// The selectable levels of a ladder, ascending.
pub fn levels<T>(rungs: &[(Effort, T)]) -> Vec<Effort> {
    rungs.iter().map(|(level, _)| *level).collect()
}

/// Return effort levels in the canonical ladder order.
pub fn normalize_levels(mut levels: Vec<Effort>) -> Vec<Effort> {
    levels.sort_unstable();
    levels.dedup();
    levels
}

/// The exact selectable level `requested` rounds to; an empty ladder has no
/// effective effort. `levels` must ascend and be deduplicated.
pub fn resolve_level(levels: &[Effort], requested: Effort) -> Option<Effort> {
    debug_assert!(levels.windows(2).all(|w| w[0] < w[1]), "effort levels must ascend");
    round_down(levels.iter().map(|&level| (level, level)), requested)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUNGS: &[(Effort, u8)] = &[(Effort::Min, 0), (Effort::Medium, 2), (Effort::Max, 5)];

    #[test]
    fn resolve_rounds_down_to_nearest_rung() {
        assert_eq!(resolve(RUNGS, Effort::Min), 0);
        assert_eq!(resolve(RUNGS, Effort::Medium), 2);
        assert_eq!(resolve(RUNGS, Effort::High), 2, "between rungs rounds down");
        assert_eq!(resolve(RUNGS, Effort::XHigh), 2);
        assert_eq!(resolve(RUNGS, Effort::Max), 5);
    }

    #[test]
    fn resolve_floors_below_the_lowest_rung() {
        let rungs = [(Effort::Low, 1_u8), (Effort::Max, 5)];
        assert_eq!(resolve(&rungs, Effort::Min), 1);
    }

    #[test]
    fn resolve_is_monotone_over_the_full_scale() {
        let mut last = resolve(RUNGS, Effort::Min);
        for level in Effort::ALL {
            let value = resolve(RUNGS, level);
            assert!(value >= last, "resolve must not decrease at {level}");
            last = value;
        }
    }

    #[test]
    fn resolve_level_returns_an_exact_selectable_level() {
        let levels = [Effort::Low, Effort::Medium, Effort::Max];
        assert_eq!(resolve_level(&levels, Effort::Medium), Some(Effort::Medium));
        assert_eq!(resolve_level(&levels, Effort::XHigh), Some(Effort::Medium));
        assert_eq!(resolve_level(&levels, Effort::Min), Some(Effort::Low));
        assert_eq!(resolve_level(&[], Effort::High), None);
    }

    /// A wire that spells every level.
    const SPELLING: &[(Effort, u8)] = &[
        (Effort::Min, 0),
        (Effort::Low, 1),
        (Effort::Medium, 2),
        (Effort::High, 3),
        (Effort::XHigh, 4),
        (Effort::Max, 5),
    ];

    #[test]
    fn resolve_among_rounds_onto_the_levels_the_model_takes() {
        let levels = [Effort::Low, Effort::Max];
        assert_eq!(offered_levels(SPELLING, &levels), levels);
        assert_eq!(resolve_among(SPELLING, &levels, Effort::Max), Some(5));
        assert_eq!(resolve_among(SPELLING, &levels, Effort::Low), Some(1));
        // Below the lowest level, as internal calls ask for, the lowest applies.
        assert_eq!(resolve_among(SPELLING, &levels, Effort::Min), Some(1));
        // Between levels, the request rounds down.
        assert_eq!(resolve_among(SPELLING, &levels, Effort::High), Some(1));
        // A model that takes every level resolves like the spelling itself.
        assert_eq!(resolve_among(SPELLING, &Effort::ALL, Effort::High), Some(3));
        // A model that takes no level sends nothing.
        assert_eq!(resolve_among(SPELLING, &[], Effort::High), None);
    }

    #[test]
    fn resolve_among_skips_levels_the_wire_cannot_spell() {
        let spelling = &SPELLING[..4];
        let levels = [Effort::Medium, Effort::Max];
        assert_eq!(offered_levels(spelling, &levels), vec![Effort::Medium]);
        assert_eq!(resolve_among(spelling, &levels, Effort::Max), Some(2));
        assert_eq!(resolve_among(spelling, &[Effort::Max], Effort::Max), None);
    }

    #[test]
    fn normalize_levels_sorts_and_deduplicates() {
        assert_eq!(
            normalize_levels(vec![Effort::High, Effort::Low, Effort::High]),
            vec![Effort::Low, Effort::High]
        );
    }
}
