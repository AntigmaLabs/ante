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
