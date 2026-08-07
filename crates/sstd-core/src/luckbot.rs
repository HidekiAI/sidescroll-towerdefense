use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Movement mode of a companion bot (LuckBot, repairer, guard, etc).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MoveMode {
    /// Roam a fixed area (an origin point + radius).
    Patrol,
    /// Follow a target entity (e.g. a catapult or a guard tower).
    Follow,
}

/// A companion bot: the agent that carries a luck stat and applies an aura.
/// Plain struct (no serde) — the game bridge maps its ids explicitly.
#[derive(Clone, Debug, PartialEq)]
pub struct Bot {
    pub id: String,
    /// Current movement instruction — where it roams or what it follows.
    pub move_mode: MoveMode,
    /// Patrol anchor (ignored when `move_mode == Follow`).
    pub patrol_x: f32,
    pub patrol_y: f32,
    /// Follow target for `Follow` mode (an entity id, resolved by the engine).
    pub follow_target: String,
    /// The bot's luck stat. Higher = better odds in its aura.
    pub luck: i32,
}

impl Bot {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            move_mode: MoveMode::Patrol,
            patrol_x: 0.0,
            patrol_y: 0.0,
            follow_target: String::new(),
            luck: 0,
        }
    }

    pub fn with_luck(mut self, luck: i32) -> Self {
        self.luck = luck;
        self
    }

    pub fn patrolling(mut self, x: f32, y: f32) -> Self {
        self.move_mode = MoveMode::Patrol;
        self.patrol_x = x;
        self.patrol_y = y;
        self
    }

    pub fn following(mut self, target: &str) -> Self {
        self.move_mode = MoveMode::Follow;
        self.follow_target = target.to_string();
        self
    }
}

/// All tunable numbers for the LuckBot luck system. Sourced from
/// `sstd_config.sqlite3` via the population system. `Default` is only a
/// fallback for an unseeded store; the game always loads from config.
///
/// Luck improves the **odds** of favorable rolls (crits, rarer drops). It
/// never removes the element of chance — fun factor keeps the dice in play.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LuckConfig {
    /// Neutral (no-effect) luck level; entities without a stat use this.
    pub default_luck: i32,
    /// Multiplier added per luck point (0.15 -> +15% per point).
    pub bonus_per_point: f64,
    /// Hard ceiling on any entity's luck stat.
    pub max_luck: i32,
    /// Cap on the resulting crit probability (never above 1.0).
    pub crit_cap: f64,
    /// Radius (in tiles) over which a bot's aura applies.
    pub aura_radius: f32,
}

impl Default for LuckConfig {
    fn default() -> Self {
        Self {
            default_luck: 0,
            bonus_per_point: 0.15,
            max_luck: 5,
            crit_cap: 1.0,
            aura_radius: 3.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Luck stat
// ---------------------------------------------------------------------------

/// Clamp a luck value to `[0 .. max_luck]` so out-of-range data can never
/// produce a runaway roll.
pub fn clamp_luck(raw: i32, config: &LuckConfig) -> i32 {
    raw.clamp(0, config.max_luck)
}

/// The additive-on-base luck multiplier for a luck level:
/// `1 + clamp(luck,0,max) * bonus_per_point`. Neutral (0) returns exactly 1.0.
pub fn luck_multiplier(luck: i32, config: &LuckConfig) -> f64 {
    1.0 + clamp_luck(luck, config) as f64 * config.bonus_per_point
}

// ---------------------------------------------------------------------------
// Crit roll
// ---------------------------------------------------------------------------

/// Resulting crit probability: `base_chance * multiplier`, capped at
/// `crit_cap`. The engine still rolls the crit; luck only raises its odds.
pub fn crit_chance(base: f64, luck: i32, config: &LuckConfig) -> f64 {
    let mult = luck_multiplier(luck, config);
    (base * mult).clamp(0.0, config.crit_cap.clamp(0.0, 1.0))
}

// ---------------------------------------------------------------------------
// Rarity roll (weighted table, luck inflates rarer weights)
// ---------------------------------------------------------------------------

/// A drop-table tier: its name plus an occurrence weight before luck.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RarityTier {
    pub name: &'static str,
    pub base_weight: u32,
}

/// Default drop table (common..legendary). Weights are code defaults; tune via
/// config (`drop.*` keys, see Open Questions) if a scenario overrides them.
pub const DEFAULT_RARITIES: [RarityTier; 5] = [
    RarityTier {
        name: "common",
        base_weight: 60,
    },
    RarityTier {
        name: "uncommon",
        base_weight: 28,
    },
    RarityTier {
        name: "rare",
        base_weight: 9,
    },
    RarityTier {
        name: "epic",
        base_weight: 2,
    },
    RarityTier {
        name: "legendary",
        base_weight: 1,
    },
];

/// The result of weighting a drop table under a luck multiplier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeightedTable<'a> {
    /// Effective (not normalized) weight per tier, parallel to `tiers`.
    pub weights: Vec<u32>,
    /// Sum of `weights`; feed `roll % total` into `pick`.
    pub total: u64,
    tiers: &'a [RarityTier],
}

impl WeightedTable<'_> {
    /// Turn a random value in `[0, total)` into the chosen tier index.
    pub fn pick(&self, roll: u64) -> Option<usize> {
        if self.tiers.is_empty() {
            return None;
        }
        let mut acc: u64 = 0;
        for (idx, w) in self.weights.iter().enumerate() {
            acc += *w as u64;
            if roll < acc {
                return Some(idx);
            }
        }
        None
    }
}

/// Apply the luck multiplier to every tier weight. Higher luck inflates the
/// rarer tiers' share of the total so a single die roll lands up-table more
/// often; the roll itself still happens.
pub fn luck_weighted_rarity<'a>(
    tiers: &'a [RarityTier],
    luck: i32,
    config: &LuckConfig,
) -> WeightedTable<'a> {
    let mult = luck_multiplier(luck, config);
    let weights: Vec<u32> = tiers
        .iter()
        .map(|t| ((t.base_weight as f64) * mult).round() as u32)
        .collect();
    let total = weights.iter().fold(0u64, |acc, w| acc + *w as u64);
    WeightedTable {
        weights,
        total,
        tiers,
    }
}

// ---------------------------------------------------------------------------
// Aura distance helper
// ---------------------------------------------------------------------------

/// Euclidean distance between two points in tiles.
pub fn distance(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt()
}

/// Whether an ally at `(x, y)` lies inside the bot's aura (measured from its
/// move anchor — patrol center, or the current follow waypoint).
pub fn within_aura(bot: &Bot, x: f32, y: f32, config: &LuckConfig) -> bool {
    let (cx, cy) = bot.move_anchor();
    distance(cx, cy, x, y) <= config.aura_radius
}

impl Bot {
    pub fn move_anchor(&self) -> (f32, f32) {
        (self.patrol_x, self.patrol_y)
    }
}

// ---------------------------------------------------------------------------
// Seeded, replayable rolls
// ---------------------------------------------------------------------------

/// A deterministic, dependency-free PRNG (splitmix64) that turns a
/// `(seed, roll_count)` pair into the same `u64` every time. Persisting only
/// the seed reproduces every successive roll, so the same seed always yields
/// the same rarity tiers / crits / stream checksum.
pub fn seeded_roll(seed: u64, counter: u64) -> u64 {
    let mut z = seed.wrapping_add(counter.wrapping_mul(0x9E37_79B9_7F4A_7C15)) ^ 0x09_6D_2C_00_00;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The replay invariant in a single number. Feed the same `seed`, replay the
/// same number of rolls, and the checksum — the running folded sum of every
/// roll — must match. Any drift (call-order change, missing roll, PRNG swap)
/// breaks the checksum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollLog {
    seed: u64,
    counter: u64,
    checksum: u64,
}

impl RollLog {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            counter: 0,
            checksum: 0,
        }
    }

    /// Advance the stream one roll, returning the rolled value and folding it
    /// into the checksum.
    pub fn roll(&mut self) -> u64 {
        let roll = seeded_roll(self.seed, self.counter);
        self.counter += 1;
        self.checksum = self.checksum.wrapping_add(roll).rotate_left(7);
        roll
    }

    pub fn counter(&self) -> u64 {
        self.counter
    }

    /// The folded checksum of every roll produced so far. Two runs with the
    /// same seed and same number of `next()` calls must agree here.
    pub fn checksum(&self) -> u64 {
        self.checksum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> LuckConfig {
        LuckConfig::default()
    }

    #[test]
    fn neutral_luck_is_no_effect() {
        let c = cfg();
        assert_eq!(luck_multiplier(0, &c), 1.0);
        assert_eq!(crit_chance(0.2, 0, &c), 0.2);
        let neutral = luck_weighted_rarity(&DEFAULT_RARITIES, 0, &c);
        let sum: u64 = DEFAULT_RARITIES.iter().map(|t| t.base_weight as u64).sum();
        assert_eq!(neutral.total, sum);
    }

    #[test]
    fn multiplier_is_additive_on_base() {
        let c = cfg();
        let m1 = luck_multiplier(1, &c);
        let m2 = luck_multiplier(2, &c);
        assert!((m1 - 1.15).abs() < 1e-9);
        assert!((m2 - 1.30).abs() < 1e-9);
    }

    #[test]
    fn luck_is_clamped_to_max() {
        let c = cfg();
        assert_eq!(clamp_luck(9999, &c), c.max_luck);
        assert_eq!(clamp_luck(-1, &c), 0);
    }

    #[test]
    fn crit_chance_scales_base_and_is_capped() {
        let c = cfg();
        assert!((crit_chance(0.2, 1, &c) - 0.23).abs() < 1e-9);
        assert!(crit_chance(0.9, 99, &c) <= 1.0);
        assert!(crit_chance(-0.1, 1, &c) >= 0.0);
    }

    #[test]
    fn rarity_shift_bisects_up_with_luck() {
        let c = cfg();
        let neutral = luck_weighted_rarity(&DEFAULT_RARITIES, 0, &c);
        let lucky = luck_weighted_rarity(&DEFAULT_RARITIES, 5, &c);
        let epic_n = DEFAULT_RARITIES[3].base_weight as f64 / neutral.total as f64;
        let epic_l = lucky.weights[3] as f64 / lucky.total as f64;
        assert!(epic_l > epic_n, "luck should inflate rarer tiers");
    }

    #[test]
    fn weighted_table_pick() {
        let c = cfg();
        let table = luck_weighted_rarity(&DEFAULT_RARITIES, 0, &c);
        assert_eq!(table.pick(0).unwrap(), 0, "roll 0 -> common");
        assert!(luck_weighted_rarity(&[], 0, &c).pick(0).is_none());
    }

    #[test]
    fn bot_move_modes() {
        let patrol = Bot::new("b1").patrolling(10.0, 20.0);
        assert_eq!(patrol.move_mode, MoveMode::Patrol);
        let follow = Bot::new("b2").following("tower_3");
        assert_eq!(follow.move_mode, MoveMode::Follow);
        assert_eq!(follow.follow_target, "tower_3");
    }

    #[test]
    fn aura_geometry() {
        let c = cfg();
        let b = Bot::new("b3").with_luck(2).patrolling(0.0, 0.0);
        assert!(within_aura(&b, 1.0, 1.0, &c), "inside radius");
        assert!(!within_aura(&b, 50.0, 50.0, &c), "outside radius");
        assert_eq!(b.move_anchor(), (0.0, 0.0));
    }

    #[test]
    fn seeded_roll_reproduces_sequence() {
        let mut a = RollLog::new(42);
        assert_eq!(seeded_roll(42, 0), a.roll());
        // A second stream from the same seed produces the identical rolls.
        let mut c = RollLog::new(42);
        let mut d = RollLog::new(42);
        for _ in 0..100 {
            assert_eq!(c.roll(), d.roll());
        }
        assert_eq!(c.checksum(), d.checksum());
    }

    #[test]
    fn replay_preserves_checksum() {
        // Save the seed, replay: same seed + same roll count => same checksum.
        let seed: u64 = 0xDEAD_BEEF_CAFE_F00D;
        let mut run1 = RollLog::new(seed);
        for _ in 0..50 {
            let _ = run1.roll();
        }
        let saved_checksum = run1.checksum();

        let mut run2 = RollLog::new(seed);
        for _ in 0..50 {
            let _ = run2.roll();
        }
        assert_eq!(run1.counter(), run2.counter());
        assert_eq!(saved_checksum, run2.checksum());
    }

    #[test]
    fn different_seed_differs() {
        let mut a = RollLog::new(1);
        let mut b = RollLog::new(2);
        let mut diverged = false;
        for _ in 0..10 {
            if a.roll() != b.roll() {
                diverged = true;
                break;
            }
        }
        assert!(diverged, "distinct seeds should produce distinct streams");
    }

    #[test]
    fn missing_roll_breaks_checksum() {
        let seed = 7;
        let mut full = RollLog::new(seed);
        for _ in 0..10 {
            let _ = full.roll();
        }
        let mut short = RollLog::new(seed);
        for _ in 0..9 {
            let _ = short.roll();
        }
        assert_ne!(full.checksum(), short.checksum());
    }
}
