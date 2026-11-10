// Pure Water Condensator arithmetic for the Industrial Foregoing slice
// (standalone crate lib, injected as `crate::game::industrialforegoing`).
//
// No game access lives here: the progress bar's countdown, the one attempt it
// fires at zero, the water that attempt condenses and the FE it costs.
//
// Upstream (`WaterCondensatorTile` and its `IndustrialWorkingTile` base, read
// from Industrial Foregoing `1.21-3.6.39`) counts the water source blocks on
// the six sides, and once the bar completes: with fewer than two sources,
// nothing; with two or more and at least 1,000 FE in the buffer, `sources *
// 100` mB for 1,000 FE; with two or more and less than that, `sources * 50`
// mB for free. The bar's next value is `floor(100 * workAmount)`, so the
// cooldown after an attempt is 10 ticks when powered, 50 when not, and 100
// when idle.

/// `WaterCondensatorConfig.maxStoredPower` default: the FE buffer.
pub const CAPACITY_FE: i32 = 10_000;
/// `WaterCondensatorConfig.maxWaterTankSize` default: the tank.
pub const TANK_CAPACITY_MB: i32 = 16_000;
/// `WaterCondensatorConfig.maxProgress` default: one bar.
pub const MAX_PROGRESS: i32 = 100;
/// `WaterCondensatorConfig.powerPerOperation` default: FE per attempt.
pub const POWER_PER_OPERATION: i32 = 1_000;
/// Water one source block adds on a powered attempt.
pub const WATER_PER_SOURCE_MB: i32 = 100;
/// Water one source block adds on an unpowered attempt.
pub const WATER_PER_SOURCE_UNPOWERED_MB: i32 = 50;
/// `getWaterSources() >= 2`: the machine does nothing below this.
pub const MIN_SOURCES: u32 = 2;

/// One completed bar's attempt, as `work()` answers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attempt {
    /// Water the attempt would condense (before the tank's capacity).
    pub water_mb: i32,
    /// FE the attempt costs.
    pub energy_fe: i32,
    /// The bar's value after the attempt, `floor(MAX_PROGRESS * workAmount)`.
    pub cooldown: i32,
}

/// What one server tick did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TickOutcome {
    /// The bar completed and `work()` ran.
    pub attempted: bool,
    /// Water actually added (the tank's capacity caps it).
    pub water_added_mb: i32,
    /// FE actually spent.
    pub energy_spent_fe: i32,
}

impl Attempt {
    /// `WorkAction` for an attempt with `sources` water sources and `energy`
    /// FE stored.
    #[must_use]
    pub fn new(sources: u32, energy: i32) -> Self {
        if sources < MIN_SOURCES {
            return Self {
                water_mb: 0,
                energy_fe: 0,
                cooldown: MAX_PROGRESS,
            };
        }
        if energy >= POWER_PER_OPERATION {
            Self {
                water_mb: sources as i32 * WATER_PER_SOURCE_MB,
                energy_fe: POWER_PER_OPERATION,
                cooldown: cooldown(1, 10),
            }
        } else {
            Self {
                water_mb: sources as i32 * WATER_PER_SOURCE_UNPOWERED_MB,
                energy_fe: 0,
                cooldown: cooldown(1, 2),
            }
        }
    }
}

/// `floor(MAX_PROGRESS * numerator / denominator)`, the bar value upstream
/// sets from `workAmount` (0.1, 0.5 or 1).
#[must_use]
pub fn cooldown(numerator: i32, denominator: i32) -> i32 {
    MAX_PROGRESS * numerator / denominator
}

/// The machine's saved state: the `energy` int (`native_block_entity_energy`
/// keeps it there), the tank and the bar's progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Condensator {
    /// FE in the buffer.
    pub energy: i32,
    /// Water in the tank, millibuckets.
    pub water_mb: i32,
    /// The bar, `1..=MAX_PROGRESS`; 0 is a completed bar waiting for its tick.
    pub progress: i32,
}

impl Default for Condensator {
    fn default() -> Self {
        Self {
            energy: 0,
            water_mb: 0,
            progress: MAX_PROGRESS,
        }
    }
}

impl Condensator {
    /// A machine from its saved compound; a missing or out-of-range value is
    /// clamped, and a missing bar starts full as a new block's does.
    #[must_use]
    pub fn from_saved(energy: i32, water_mb: i32, progress: Option<i32>) -> Self {
        Self {
            energy: energy.clamp(0, CAPACITY_FE),
            water_mb: water_mb.clamp(0, TANK_CAPACITY_MB),
            progress: progress.unwrap_or(MAX_PROGRESS).clamp(0, MAX_PROGRESS),
        }
    }

    /// Room left in the FE buffer.
    #[must_use]
    pub fn energy_room(&self) -> i32 {
        (CAPACITY_FE - self.energy).max(0)
    }

    /// Insert FE into the buffer, answering how much was taken (the buffer's
    /// `SimpleEnergyHandler` accepts up to its capacity).
    pub fn insert_energy(&mut self, amount: i32) -> i32 {
        let taken = amount.max(0).min(self.energy_room());
        self.energy += taken;
        taken
    }

    /// Room left in the tank.
    #[must_use]
    pub fn tank_room(&self) -> i32 {
        (TANK_CAPACITY_MB - self.water_mb).max(0)
    }

    /// One server tick: count the bar down, and run one attempt when it
    /// reaches zero.
    pub fn tick(&mut self, sources: u32) -> TickOutcome {
        let attempt = {
            self.progress = (self.progress - 1).max(0);
            if self.progress > 0 {
                return TickOutcome::default();
            }
            Attempt::new(sources, self.energy)
        };
        let added = attempt.water_mb.min(self.tank_room()).max(0);
        self.water_mb += added;
        let spent = attempt.energy_fe.min(self.energy).max(0);
        self.energy -= spent;
        self.progress = attempt.cooldown;
        TickOutcome {
            attempted: true,
            water_added_mb: added,
            energy_spent_fe: spent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A new machine, as `block_entity` makes one.
    fn fresh() -> Condensator {
        Condensator::default()
    }

    #[test]
    fn a_new_machine_is_empty_and_its_bar_is_full() {
        let machine = fresh();
        assert_eq!(machine.energy, 0);
        assert_eq!(machine.water_mb, 0);
        assert_eq!(machine.progress, MAX_PROGRESS);
    }

    #[test]
    fn the_first_attempt_is_after_one_hundred_ticks() {
        let mut machine = fresh();
        machine.energy = CAPACITY_FE;
        for _ in 0..MAX_PROGRESS - 1 {
            let outcome = machine.tick(MIN_SOURCES);
            assert!(!outcome.attempted);
        }
        assert_eq!(machine.progress, 1);
        assert_eq!(machine.water_mb, 0);
        let outcome = machine.tick(MIN_SOURCES);
        assert!(outcome.attempted);
        assert_eq!(outcome.water_added_mb, 200);
    }

    #[test]
    fn a_powered_attempt_condenses_one_hundred_a_source() {
        let mut machine = Condensator {
            energy: POWER_PER_OPERATION,
            ..Condensator {
                progress: 1,
                ..fresh()
            }
        };
        let outcome = machine.tick(2);
        assert_eq!(outcome.water_added_mb, 200);
        assert_eq!(outcome.energy_spent_fe, POWER_PER_OPERATION);
        assert_eq!(machine.water_mb, 200);
        assert_eq!(machine.energy, 0);
    }

    #[test]
    fn one_source_condenses_nothing() {
        let mut machine = Condensator {
            energy: CAPACITY_FE,
            progress: 1,
            ..fresh()
        };
        let outcome = machine.tick(1);
        assert!(outcome.attempted);
        assert_eq!(outcome.water_added_mb, 0);
        assert_eq!(outcome.energy_spent_fe, 0);
        assert_eq!(machine.water_mb, 0);
        assert_eq!(machine.energy, CAPACITY_FE);
    }

    #[test]
    fn no_sources_condense_nothing() {
        let mut machine = Condensator {
            energy: CAPACITY_FE,
            progress: 1,
            ..fresh()
        };
        let outcome = machine.tick(0);
        assert_eq!(outcome.water_added_mb, 0);
        assert_eq!(outcome.energy_spent_fe, 0);
    }

    #[test]
    fn an_unpowered_attempt_condenses_half_for_free() {
        let mut machine = Condensator {
            energy: POWER_PER_OPERATION - 1,
            progress: 1,
            ..fresh()
        };
        let outcome = machine.tick(2);
        assert_eq!(outcome.water_added_mb, 100);
        assert_eq!(outcome.energy_spent_fe, 0);
        assert_eq!(machine.energy, POWER_PER_OPERATION - 1);
    }

    #[test]
    fn six_sources_condense_six_hundred() {
        let mut machine = Condensator {
            energy: CAPACITY_FE,
            progress: 1,
            ..fresh()
        };
        assert_eq!(machine.tick(6).water_added_mb, 600);
        assert_eq!(machine.water_mb, 600);
        assert_eq!(machine.energy, CAPACITY_FE - POWER_PER_OPERATION);
    }

    #[test]
    fn an_unpowered_attempt_with_four_sources_condenses_two_hundred() {
        let mut machine = Condensator {
            progress: 1,
            ..fresh()
        };
        assert_eq!(machine.tick(4).water_added_mb, 200);
    }

    #[test]
    fn the_cooldown_is_ten_powered_fifty_unpowered_and_a_hundred_idle() {
        assert_eq!(Attempt::new(2, CAPACITY_FE).cooldown, 10);
        assert_eq!(Attempt::new(2, 0).cooldown, 50);
        assert_eq!(Attempt::new(1, CAPACITY_FE).cooldown, 100);
        assert_eq!(Attempt::new(0, 0).cooldown, 100);
    }

    #[test]
    fn a_powered_operation_is_followed_by_another_one_ten_ticks_later() {
        let mut machine = Condensator {
            energy: CAPACITY_FE,
            progress: 1,
            ..fresh()
        };
        machine.tick(2);
        for _ in 0..9 {
            assert!(!machine.tick(2).attempted);
        }
        let outcome = machine.tick(2);
        assert!(outcome.attempted);
        assert_eq!(outcome.water_added_mb, 200);
    }

    #[test]
    fn an_idle_bar_waits_a_hundred_ticks() {
        let mut machine = Condensator {
            energy: CAPACITY_FE,
            progress: 1,
            ..fresh()
        };
        machine.tick(0);
        assert_eq!(machine.progress, MAX_PROGRESS);
        for _ in 0..MAX_PROGRESS - 1 {
            assert!(!machine.tick(0).attempted);
        }
        assert!(machine.tick(0).attempted);
    }

    #[test]
    fn an_unpowered_bar_waits_fifty_ticks() {
        let mut machine = Condensator {
            progress: 1,
            ..fresh()
        };
        machine.tick(2);
        assert_eq!(machine.progress, 50);
        for _ in 0..49 {
            assert!(!machine.tick(2).attempted);
        }
        assert!(machine.tick(2).attempted);
    }

    #[test]
    fn the_tank_stops_at_its_capacity_and_the_fe_is_still_spent() {
        let mut machine = Condensator {
            energy: CAPACITY_FE,
            water_mb: TANK_CAPACITY_MB - 100,
            progress: 1,
        };
        let outcome = machine.tick(2);
        assert_eq!(outcome.water_added_mb, 100);
        assert_eq!(outcome.energy_spent_fe, POWER_PER_OPERATION);
        assert_eq!(machine.water_mb, TANK_CAPACITY_MB);
        assert_eq!(machine.energy, CAPACITY_FE - POWER_PER_OPERATION);
    }

    #[test]
    fn a_full_tank_still_spends_the_fe() {
        let mut machine = Condensator {
            energy: CAPACITY_FE,
            water_mb: TANK_CAPACITY_MB,
            progress: 1,
        };
        let outcome = machine.tick(6);
        assert_eq!(outcome.water_added_mb, 0);
        assert_eq!(outcome.energy_spent_fe, POWER_PER_OPERATION);
        assert_eq!(machine.water_mb, TANK_CAPACITY_MB);
    }

    #[test]
    fn inserting_fe_stops_at_the_capacity_and_refuses_negatives() {
        let mut machine = fresh();
        assert_eq!(machine.insert_energy(-5), 0);
        assert_eq!(machine.insert_energy(4_000), 4_000);
        assert_eq!(machine.insert_energy(CAPACITY_FE), CAPACITY_FE - 4_000);
        assert_eq!(machine.energy, CAPACITY_FE);
        assert_eq!(machine.insert_energy(1), 0);
    }

    #[test]
    fn the_room_is_never_negative() {
        let machine = Condensator {
            energy: CAPACITY_FE + 1,
            water_mb: TANK_CAPACITY_MB + 1,
            progress: 0,
        };
        assert_eq!(machine.energy_room(), 0);
        assert_eq!(machine.tank_room(), 0);
    }

    #[test]
    fn from_saved_clamps_and_defaults_a_missing_bar_to_full() {
        let machine = Condensator::from_saved(-4, 999_999, None);
        assert_eq!(machine.energy, 0);
        assert_eq!(machine.water_mb, TANK_CAPACITY_MB);
        assert_eq!(machine.progress, MAX_PROGRESS);
        let machine = Condensator::from_saved(12, 7, Some(250));
        assert_eq!(machine.energy, 12);
        assert_eq!(machine.water_mb, 7);
        assert_eq!(machine.progress, MAX_PROGRESS);
    }

    #[test]
    fn the_last_fe_buys_one_powered_attempt() {
        let mut machine = Condensator {
            energy: POWER_PER_OPERATION,
            progress: 1,
            ..fresh()
        };
        assert_eq!(machine.tick(2).energy_spent_fe, POWER_PER_OPERATION);
        machine.progress = 1;
        let outcome = machine.tick(2);
        assert_eq!(outcome.energy_spent_fe, 0);
        assert_eq!(outcome.water_added_mb, 100);
    }
}
