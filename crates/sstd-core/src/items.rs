use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::luckbot::{crit_chance, LuckConfig, RollLog};

/// A piece of luck gear. The archetype is a **Ring of Luck**: while equipped it
/// (a) adds a luck factor to drop rolls, and (b) gives the wearer a chance to
/// roll an incoming critical hit back to a normal hit.
///
/// The enemy can carry the same ring: when the enemy crits (their luck raised
/// the odds), the hero's own gear rolls to downgrade it — a single die governed
/// by two luck stats.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct LuckGear {
    pub id: String,
    /// Bonus luck applied to drop rolls while equipped (adds to the wearer's
    /// base luck, same scale as `luck.default_luck`).
    pub drop_luck_bonus: i32,
    /// Chance (0..=1) to turn an inbound crit into a normal hit, *before* any
    /// per-gear/per-luck scaling. `0.0` means the item has no deflect property.
    pub crit_deflect_chance: f64,
}

/// All tunable constants for luck gear. Seeded via the population system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LuckGearConfig {
    /// Cap on the combined crit-deflect probability (0..=1).
    pub deflect_cap: f64,
    /// Additional deflect chance granted per equipped gear item.
    pub deflect_per_gear: f64,
    /// Additional deflect chance granted per point of the wearer's luck.
    pub deflect_bonus_per_luck: f64,
}

impl Default for LuckGearConfig {
    fn default() -> Self {
        Self {
            deflect_cap: 0.5,
            deflect_per_gear: 0.15,
            deflect_bonus_per_luck: 0.02,
        }
    }
}

/// Effective drop luck after equipped gear is applied (`base + gear bonuses`).
/// Feed the result straight into `luck_weighted_rarity`.
pub fn drop_luck_with_gear(luck: i32, gear: &[LuckGear]) -> i32 {
    luck + gear.iter().map(|g| g.drop_luck_bonus).sum::<i32>()
}

/// Final chance the wearer turns an inbound crit into a normal hit:
/// gear's own chance + per-item bonus + per-luck bonus, capped at `deflect_cap`.
pub fn deflect_chance(gear: &[LuckGear], luck: i32, config: &LuckGearConfig) -> f64 {
    let from_gear = gear.iter().map(|g| g.crit_deflect_chance).sum::<f64>();
    let from_count = config.deflect_per_gear * gear.len() as f64;
    let from_luck = config.deflect_bonus_per_luck * luck.max(0) as f64;
    (from_gear + from_count + from_luck).clamp(0.0, config.deflect_cap)
}

/// Result of resolving one inbound attack through both luck rolls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InboundHit {
    /// The attacker missed their crit roll — a normal hit.
    Normal,
    /// The attacker crit; the defender's gear deflected it back to a normal hit.
    Deflected,
    /// The attacker crit and the defender's deflect roll failed.
    Crit,
}

const ROLL_PERCENT: u64 = 100_000;

/// Resolve one inbound attack using the seeded `log`:
///
/// 1. Roll the attacker's crit (chance = `crit_chance(base, attacker_luck,
///    attacker_luck_config)`).
/// 2. If it crits, roll the defender's gear to deflect it back to a normal hit
///    (chance = `deflect_chance(gear, defender_luck, config)`).
///
/// Two `log.roll()` draws for replay: the same seed reproduces both outcomes.
/// `gear` & `defender_luck` belong to the DEFENDER (the ring's wearer).
#[allow(clippy::too_many_arguments)]
pub fn roll_inbound_attack(
    base_crit_chance: f64,
    attacker_luck: i32,
    attacker_config: &LuckConfig,
    gear: &[LuckGear],
    defender_luck: i32,
    config: &LuckGearConfig,
    log: &mut RollLog,
) -> InboundHit {
    let crit = crit_chance(base_crit_chance, attacker_luck, attacker_config);
    let is_crit =
        log.roll() % ROLL_PERCENT < (crit.clamp(0.0, 1.0) * (ROLL_PERCENT as f64)).round() as u64;
    if !is_crit {
        return InboundHit::Normal;
    }
    let deflect = deflect_chance(gear, defender_luck, config);
    let deflected = log.roll() % ROLL_PERCENT < (deflect * (ROLL_PERCENT as f64)).round() as u64;
    if deflected {
        InboundHit::Deflected
    } else {
        InboundHit::Crit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gear() -> LuckGearConfig {
        LuckGearConfig::default()
    }

    fn ring_of_luck() -> LuckGear {
        LuckGear {
            id: "ring_of_luck".to_string(),
            drop_luck_bonus: 1,
            crit_deflect_chance: 0.1,
        }
    }

    #[test]
    fn drop_luck_adds_gear_bonus() {
        let gear = [ring_of_luck()];
        assert_eq!(drop_luck_with_gear(2, &gear), 3);
        assert_eq!(drop_luck_with_gear(2, &[]), 2);
    }

    #[test]
    fn deflect_chance_scales_and_caps() {
        let config = gear();
        // luck 0: gear base 0.1 + per-gear 0.15 = 0.25
        let chance = deflect_chance(&[ring_of_luck()], 0, &config);
        assert!((chance - 0.25).abs() < 1e-9);
        // Two rings @ luck 5 => 0.1*2 + 0.15*2 + 0.02*5 = 0.5+0.1=0.6 -> capped 0.5
        let six = [ring_of_luck(), ring_of_luck()];
        let capped = deflect_chance(&six, 5, &config);
        assert_eq!(capped, config.deflect_cap);
    }

    #[test]
    fn inbound_attack_rolls_deflect() {
        let luck_cfg = LuckConfig::default();
        let high_deflect = LuckGearConfig {
            deflect_cap: 1.0,
            deflect_per_gear: 0.9,
            deflect_bonus_per_luck: 0.0,
        };
        let max_ring = [LuckGear {
            id: "maxring".into(),
            drop_luck_bonus: 0,
            crit_deflect_chance: 1.0,
        }];
        // 100% attacker crit, 100% deflect => every hit deflected.
        let mut log = RollLog::new(0xDEAD_BEEF);
        assert_eq!(
            roll_inbound_attack(1.0, 0, &luck_cfg, &max_ring, 1, &high_deflect, &mut log),
            InboundHit::Deflected
        );
    }

    #[test]
    fn inbound_attack_misses_crit_is_normal() {
        let luck_cfg = LuckConfig::default();
        let config = gear();
        let ring = [ring_of_luck()];
        // attacker crit chance 0 => always Normal, no second roll drawn.
        let mut log = RollLog::new(1234);
        assert_eq!(
            roll_inbound_attack(0.0, 0, &luck_cfg, &ring, 0, &config, &mut log),
            InboundHit::Normal
        );
        assert_eq!(
            log.counter(),
            1,
            "crit rolls first; no deflect draw on miss"
        );
    }

    #[test]
    fn roll_is_replayable() {
        let luck_cfg = LuckConfig::default();
        let config = gear();
        let ring = [ring_of_luck()];
        let mut a = RollLog::new(77);
        let mut b = RollLog::new(77);
        for _ in 0..200 {
            assert_eq!(
                roll_inbound_attack(0.5, 2, &luck_cfg, &ring, 3, &config, &mut a),
                roll_inbound_attack(0.5, 2, &luck_cfg, &ring, 3, &config, &mut b)
            );
        }
        assert_eq!(a.checksum(), b.checksum());
    }
}
