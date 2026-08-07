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
    /// The bot's luck stat. Higher = stronger buff in its aura.
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
/// Luck is a **predictable bounded modifier** (SSTD is strategic / no-RNG):
/// it never rolls dice. Higher luck means a strictly larger, still-bounded,
/// guaranteed boost.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LuckConfig {
    /// Neutral (no-effect) luck level; entities without a stat use this.
    pub default_luck: i32,
    /// Fraction of bonus gained per luck point (0.05 -> +5% per point).
    pub bonus_per_point: f64,
    /// Hard ceiling on any entity's luck stat.
    pub max_luck: i32,
    /// Hard ceiling on the total boost (e.g. 0.25 = cap at +25%).
    pub bonus_cap: f64,
    /// Radius (in tiles) over which a bot's aura applies.
    pub aura_radius: f32,
}

impl Default for LuckConfig {
    fn default() -> Self {
        Self {
            default_luck: 0,
            bonus_per_point: 0.15,
            max_luck: 3,
            bonus_cap: 0.25,
            aura_radius: 3.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Luck stat (deterministic, bounded)
// ---------------------------------------------------------------------------

/// Clamp a luck value to `[0 .. max_luck]` so out-of-range data can never
/// produce an unbounded boost.
pub fn clamp_luck(raw: i32, config: &LuckConfig) -> i32 {
    raw.clamp(0, config.max_luck)
}

/// The bonus fraction `min(luck * bonus_per_point, bonus_cap)`, already clamped.
/// Neutral luck (0) yields exactly `0.0` (no effect, never negative).
pub fn luck_bonus_fraction(luck: i32, config: &LuckConfig) -> f64 {
    let raw = clamp_luck(luck, config) as f64 * config.bonus_per_point;
    raw.clamp(0.0, config.bonus_cap.clamp(0.0, 1.0))
}

/// The deterministic damage multiplier: `1 + luck_bonus_fraction`.
pub fn damage_multiplier(luck: i32, config: &LuckConfig) -> f64 {
    1.0 + luck_bonus_fraction(luck, config)
}

/// Deterministic (not random) critical hit: an interval `crit` every N hits.
///
/// Returns `1` once luck crosses the `bonus_cap` (guaranteeing a crit every hit
/// at max luck), or `Some(n)` meaning "crit every n-th attack" / `None` meaning
/// never. Because the boost is a bounded fudge factor, the interval shrinks as
/// luck grows and never exceeds the configured cap.
pub fn crit_interval(luck: i32, config: &LuckConfig) -> Option<u32> {
    let bonus = luck_bonus_fraction(luck, config);
    // 0% bonus -> never crit; rising bonus compresses the interval.
    if bonus <= 0.0 {
        return None;
    }
    let interval = (1.0 / bonus).ceil() as u32;
    // Max luck (cap reached) -> every hit crits.
    if luck >= config.max_luck && bonus >= config.bonus_cap {
        Some(1)
    } else {
        Some(interval.max(2))
    }
}

// ---------------------------------------------------------------------------
// Rarity (deterministic floor, not weighted rolls)
// ---------------------------------------------------------------------------

/// Highest rarity index the player is guaranteed: luck systematically raises
/// the floor of what drops. `0` = common only; `len - 1` = guaranteed top tier.
/// Non-random: a given luck always yields the same minimum tier.
pub fn rarity_floor(luck: i32, config: &LuckConfig, tier_count: usize) -> usize {
    if tier_count == 0 {
        return 0;
    }
    let level = clamp_luck(luck, config) as usize;
    let max_tier = tier_count - 1;
    // +max luck bumps every remaining tier; scaling is linear and bounded.
    let floor = level * max_tier / config.max_luck.max(1) as usize;
    floor.min(max_tier)
}

// ---------------------------------------------------------------------------
// Aura (mode-agnostic distance helper for the engine)
// ---------------------------------------------------------------------------

/// Effective distance between two points in tiles.
pub fn distance(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt()
}

/// Whether an ally at `(x, y)` lies inside the bot's aura. The core only
/// computes geometry; the engine decides which allies count as in-aura by
/// calling this with the bot's patrol or follow anchor.
pub fn within_aura(bot: &Bot, x: f32, y: f32, config: &LuckConfig) -> bool {
    let (cx, cy) = bot.move_anchor();
    distance(cx, cy, x, y) <= config.aura_radius
}

impl Bot {
    /// The anchor the aura is measured from: the patrol center, or the follow
    /// waypoint (the engine updates `patrol_x/y` as the followed target moves).
    pub fn move_anchor(&self) -> (f32, f32) {
        (self.patrol_x, self.patrol_y)
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
        assert_eq!(luck_bonus_fraction(0, &c), 0.0);
        assert!((damage_multiplier(0, &c) - 1.0).abs() < 1e-9);
        assert_eq!(crit_interval(0, &c), None);
        assert_eq!(rarity_floor(0, &c, 5), 0);
    }

    #[test]
    fn deterministic_ranges() {
        let c = cfg();
        assert_eq!(clamp_luck(999, &c), c.max_luck);
        assert_eq!(clamp_luck(-5, &c), 0);
        assert!(luck_bonus_fraction(999, &c) <= c.bonus_cap);
        assert!(luck_bonus_fraction(1, &c) > 0.0);
    }

    #[test]
    fn crit_interval_shortens_with_luck() {
        let c = cfg();
        let lepi = crit_interval(1, &c).unwrap();
        let hip = crit_interval(2, &c).unwrap();
        assert!(hip < lepi, "more luck should crit more often");
        // Max luck guarantees a crit every hit.
        assert_eq!(crit_interval(c.max_luck, &c), Some(1));
    }

    #[test]
    fn rarity_floor_is_monotonic() {
        let c = cfg();
        let low = rarity_floor(1, &c, 5);
        let high = rarity_floor(c.max_luck, &c, 5);
        assert!(high >= low);
        assert_eq!(high, 4); // max luck -> top tier
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
}
