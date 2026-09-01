//! Effort ladder resolution shared by provider profiles.

use ante_protocol_shape::Effort;

/// Round `requested` down to the nearest rung's setting; below the lowest
/// rung, the lowest applies. `rungs` must be non-empty and sorted ascending by level.
pub fn resolve<T: Copy>(rungs: &[(Effort, T)], requested: Effort) -> T {
    debug_assert!(rungs.windows(2).all(|w| w[0].0 < w[1].0), "ladder rungs must ascend");
    let mut setting = rungs[0].1;
    for &(level, rung_setting) in rungs {
        if level > requested {
            break;
        }
        setting = rung_setting;
    }
    setting
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

/// Resolve a requested effort to an exact selectable level. Requests between
/// levels round down; requests below the floor take the floor. An empty ladder
/// has no effective effort. `levels` must be sorted ascending and deduplicated.
pub fn resolve_level(levels: &[Effort], requested: Effort) -> Option<Effort> {
    debug_assert!(levels.windows(2).all(|w| w[0] < w[1]), "effort levels must ascend");
    let mut selected = *levels.first()?;
    for &level in levels {
        if level > requested {
            break;
        }
        selected = level;
    }
    Some(selected)
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

    #[test]
    fn normalize_levels_sorts_and_deduplicates() {
        assert_eq!(
            normalize_levels(vec![Effort::High, Effort::Low, Effort::High]),
            vec![Effort::Low, Effort::High]
        );
    }
}
