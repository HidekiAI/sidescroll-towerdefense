use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Persistent progression state of a hero that can die. Kept free of serde so
/// the game bridge maps it explicitly; the option/artifact enums below ARE
/// serializable because they travel through the editor UI.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerProfile {
    pub level: u32,
    pub exp: u64,
    pub current_hp: i32,
    pub max_hp: i32,
    pub current_mp: i32,
    pub max_mp: i32,
    pub maseki: u64,
    pub bot_lives: u32,
}

impl PlayerProfile {
    /// Restore HP and MP to their maxima (the baseline "revived" state).
    pub fn full_restore(&self) -> Self {
        Self {
            current_hp: self.max_hp,
            current_mp: self.max_mp,
            ..self.clone()
        }
    }

    /// Zero out Exp without touching level or resources.
    pub fn zero_exp(&self) -> Self {
        Self {
            exp: 0,
            ..self.clone()
        }
    }

    /// Clamp current HP to the given "leave me alive" value (used by the
    /// damage-absorb artifact: the killing hit instead leaves 1 HP).
    pub fn at_leave_hp(&self, leave_hp: i32) -> Self {
        Self {
            current_hp: leave_hp.clamp(1, self.max_hp),
            ..self.clone()
        }
    }
}

/// The kinds of auto-resurrect artifacts the player may own.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutoResurrectKind {
    /// Free resurrection — full restore, no cost.
    NoPenalty,
    /// Costs a number of bot lives (default from config when `count == 0`).
    CostBotLives { count: u32 },
    /// Transfers X% of current MP to HP (100 MP @ 50% -> 50 HP).
    MpToHp { percent: u32 },
    /// Pay a fixed amount of maseki.
    PayMaseki { amount: u64 },
    /// Pay N% of owned maseki, but never less than a minimum payment.
    PayMasekiPercent { percent: u32, min_amount: u64 },
}

/// Every choice the countdown dialog can offer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReviveOption {
    /// Revive immediately but lose levels (`levels == 0` -> use config default).
    LoseLevel { levels: u32 },
    /// Restart the session; keep level, reset Exp to 0.
    ResetExp,
    /// Restart the scenario with no penalty.
    RestartScenario,
    /// Damage-absorb artifact: survive the hit at leave-HP, then convert
    /// N MP -> M HP each turn while MP remain.
    DamageAbsorb { mp_per_turn: u32, hp_per_turn: u32 },
    /// Consume an owned auto-resurrect artifact.
    AutoResurrect { artifact: AutoResurrectKind },
}

/// All tunable numbers for the death/revive system. Sourced from
/// `sstd_config.sqlite3` via the population system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReviveConfig {
    pub lose_level_cost: u32,
    pub damage_absorb_leave_hp: i32,
    pub damage_absorb_mp_per_turn: u32,
    pub damage_absorb_hp_per_turn: u32,
    pub auto_resurrect_bot_lives: u32,
    pub auto_resurrect_mp_to_hp_percent: u32,
    pub auto_resurrect_pay_maseki_amount: u64,
    pub auto_resurrect_pay_maseki_percent: u32,
    pub auto_resurrect_pay_maseki_min: u64,
    pub dialog_countdown_secs: u32,
}

impl Default for ReviveConfig {
    fn default() -> Self {
        Self {
            lose_level_cost: 1,
            damage_absorb_leave_hp: 1,
            damage_absorb_mp_per_turn: 10,
            damage_absorb_hp_per_turn: 10,
            auto_resurrect_bot_lives: 5,
            auto_resurrect_mp_to_hp_percent: 50,
            auto_resurrect_pay_maseki_amount: 100,
            auto_resurrect_pay_maseki_percent: 10,
            auto_resurrect_pay_maseki_min: 50,
            dialog_countdown_secs: 10,
        }
    }
}

/// Why a revive attempt failed. `NoLevels` is the only permanent-death path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviveError {
    NoLevels { cost: u32, level: u32 },
    NoBotLives { required: u32, owned: u32 },
    InsufficientMp { required: u32, owned: u32 },
    InsufficientMaseki { required: u64, owned: u64 },
}

/// Result of a single countdown-dialog session.
#[derive(Clone, Debug, PartialEq)]
pub enum ReviveOutcome {
    Revived {
        profile: PlayerProfile,
        option: ReviveOption,
    },
    Declined {
        reason: DeclineReason,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeclineReason {
    /// Player chose "accept death".
    PlayerDeclined,
    /// Countdown expired with no selection.
    Expired,
    /// The chosen option was unaffordable.
    Error(ReviveError),
}

/// What the player did with the countdown dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviveChoice {
    Revive(ReviveOption),
    None,
}

/// Countdown-based modal dialog. Pure state; the game drives it with `tick()`
/// each frame/tick and reads `selection` when `is_resolved()`.
#[derive(Clone, Debug, PartialEq)]
pub struct ReviveDialog {
    pub options: Vec<ReviveOption>,
    pub deadline_ticks: u32,
    pub remaining_ticks: u32,
    pub selection: Option<ReviveChoice>,
}

impl ReviveDialog {
    pub fn new(options: Vec<ReviveOption>, countdown_ticks: u32) -> Self {
        Self {
            options,
            deadline_ticks: countdown_ticks,
            remaining_ticks: countdown_ticks,
            selection: None,
        }
    }

    /// Advance one tick. Returns `true` the moment the countdown expires with
    /// no selection (the player is treated as declining / quitting).
    pub fn tick(&mut self) -> bool {
        if self.selection.is_some() {
            return false;
        }
        if self.remaining_ticks > 0 {
            self.remaining_ticks -= 1;
        }
        if self.remaining_ticks == 0 {
            self.selection = Some(ReviveChoice::None);
            return true;
        }
        false
    }

    /// Lock in a choice. Only the first choice is honored.
    pub fn choose(&mut self, choice: ReviveChoice) {
        if self.selection.is_none() {
            self.selection = Some(choice);
        }
    }

    pub fn is_resolved(&self) -> bool {
        self.selection.is_some()
    }
}

// ---------------------------------------------------------------------------
// Pure resolution
// ---------------------------------------------------------------------------

/// Resolve one revive option against a profile and the current config.
pub fn resolve_revive(
    option: &ReviveOption,
    profile: &PlayerProfile,
    config: &ReviveConfig,
) -> Result<PlayerProfile, ReviveError> {
    match option {
        ReviveOption::LoseLevel { levels } => {
            let cost = if *levels == 0 {
                config.lose_level_cost
            } else {
                *levels
            };
            revive_lose_level(profile, cost)
        }
        ReviveOption::ResetExp => Ok(profile.zero_exp().full_restore()),
        ReviveOption::RestartScenario => Ok(profile.full_restore()),
        ReviveOption::DamageAbsorb { .. } => {
            // Survives the hit at leave-HP; the per-turn MP->HP conversion is a
            // separate passive applied by damage_absorb_turn()/apply().
            Ok(profile.at_leave_hp(config.damage_absorb_leave_hp))
        }
        ReviveOption::AutoResurrect { artifact } => {
            resolve_auto_resurrect(artifact, profile, config)
        }
    }
}

/// Option 1: revive but drop `cost` levels. `cost > level` (including level 0)
/// means permanent death — no negative levels.
pub fn revive_lose_level(profile: &PlayerProfile, cost: u32) -> Result<PlayerProfile, ReviveError> {
    if profile.level < cost {
        return Err(ReviveError::NoLevels {
            cost,
            level: profile.level,
        });
    }
    Ok(PlayerProfile {
        level: profile.level - cost,
        ..profile.full_restore()
    })
}

/// Resolve an owned auto-resurrect artifact.
pub fn resolve_auto_resurrect(
    kind: &AutoResurrectKind,
    profile: &PlayerProfile,
    config: &ReviveConfig,
) -> Result<PlayerProfile, ReviveError> {
    match kind {
        AutoResurrectKind::NoPenalty => Ok(profile.full_restore()),
        AutoResurrectKind::CostBotLives { count } => {
            let required = if *count == 0 {
                config.auto_resurrect_bot_lives
            } else {
                *count
            };
            if profile.bot_lives < required {
                return Err(ReviveError::NoBotLives {
                    required,
                    owned: profile.bot_lives,
                });
            }
            Ok(PlayerProfile {
                bot_lives: profile.bot_lives - required,
                ..profile.full_restore()
            })
        }
        AutoResurrectKind::MpToHp { percent } => {
            let pct = if *percent == 0 {
                config.auto_resurrect_mp_to_hp_percent
            } else {
                *percent
            };
            let hp_gained = mp_to_hp(profile.current_mp, pct);
            if hp_gained <= 0 {
                return Err(ReviveError::InsufficientMp {
                    required: 1,
                    owned: profile.current_mp.max(0) as u32,
                });
            }
            Ok(PlayerProfile {
                current_mp: profile.current_mp - hp_gained,
                current_hp: (profile.current_hp + hp_gained).min(profile.max_hp),
                ..profile.clone()
            })
        }
        AutoResurrectKind::PayMaseki { amount } => {
            let required = if *amount == 0 {
                config.auto_resurrect_pay_maseki_amount
            } else {
                *amount
            };
            if profile.maseki < required {
                return Err(ReviveError::InsufficientMaseki {
                    required,
                    owned: profile.maseki,
                });
            }
            Ok(PlayerProfile {
                maseki: profile.maseki - required,
                ..profile.full_restore()
            })
        }
        AutoResurrectKind::PayMasekiPercent {
            percent,
            min_amount,
        } => {
            let pct = if *percent == 0 {
                config.auto_resurrect_pay_maseki_percent
            } else {
                *percent
            };
            let minimum = if *min_amount == 0 {
                config.auto_resurrect_pay_maseki_min
            } else {
                *min_amount
            };
            let payment = maseki_percent_payment(profile.maseki, pct, minimum);
            if profile.maseki < payment {
                return Err(ReviveError::InsufficientMaseki {
                    required: payment,
                    owned: profile.maseki,
                });
            }
            Ok(PlayerProfile {
                maseki: profile.maseki - payment,
                ..profile.full_restore()
            })
        }
    }
}

/// Convert a percentage of MP into HP. 100 MP @ 50% -> 50 HP.
pub fn mp_to_hp(mp: i32, percent: u32) -> i32 {
    (mp.max(0) as i64 * percent as i64 / 100) as i32
}

/// Flat maseki payment for a percentage-of-owned cost with a floor:
/// `max(floor(owned * N / 100), min_amount)`.
pub fn maseki_percent_payment(owned: u64, percent: u32, min_amount: u64) -> u64 {
    let pct = (owned * percent as u64) / 100;
    pct.max(min_amount)
}

/// One damage-absorb tick: spend N MP to gain M HP (only while MP remain).
pub fn damage_absorb_turn(profile: &PlayerProfile, config: &ReviveConfig) -> DamageAbsorbTurn {
    let mp_needed = config.damage_absorb_mp_per_turn as i32;
    if profile.current_mp < mp_needed {
        return DamageAbsorbTurn {
            mp_spent: 0,
            hp_gained: 0,
        };
    }
    DamageAbsorbTurn {
        mp_spent: config.damage_absorb_mp_per_turn,
        hp_gained: config.damage_absorb_hp_per_turn as i32,
    }
}

/// Apply one damage-absorb tick to the profile.
pub fn apply_damage_absorb_turn(profile: &PlayerProfile, config: &ReviveConfig) -> PlayerProfile {
    let turn = damage_absorb_turn(profile, config);
    PlayerProfile {
        current_mp: profile.current_mp - turn.mp_spent as i32,
        current_hp: (profile.current_hp + turn.hp_gained).min(profile.max_hp),
        ..profile.clone()
    }
}

/// Whether a revive option is affordable given the current profile.
pub fn is_affordable(
    option: &ReviveOption,
    profile: &PlayerProfile,
    config: &ReviveConfig,
) -> bool {
    resolve_revive(option, profile, config).is_ok()
}

// ---------------------------------------------------------------------------
// Result of one damage-absorb turn
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamageAbsorbTurn {
    pub mp_spent: u32,
    pub hp_gained: i32,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> PlayerProfile {
        PlayerProfile {
            level: 5,
            exp: 400,
            current_hp: 10,
            max_hp: 100,
            current_mp: 100,
            max_mp: 100,
            maseki: 1000,
            bot_lives: 5,
        }
    }

    #[test]
    fn full_restore_and_zero_exp() {
        let p = profile();
        let restored = p.full_restore();
        assert_eq!(restored.current_hp, 100);
        assert_eq!(restored.current_mp, 100);
        let zeroed = p.zero_exp();
        assert_eq!(zeroed.exp, 0);
        assert_eq!(zeroed.level, 5);
    }

    #[test]
    fn revive_lose_level_succeeds() {
        let out = revive_lose_level(&profile(), 1).unwrap();
        assert_eq!(out.level, 4);
        assert_eq!(out.current_hp, 100, "revived at full HP");
        assert_eq!(out.exp, 400, "exp untouched");
    }

    #[test]
    fn revive_lose_level_permanent_death_at_zero() {
        let mut p = profile();
        p.level = 0;
        let err = revive_lose_level(&p, 1).unwrap_err();
        assert_eq!(err, ReviveError::NoLevels { cost: 1, level: 0 });
    }

    #[test]
    fn revive_lose_level_permanent_death_when_below_cost() {
        let mut p = profile();
        p.level = 1;
        let err = revive_lose_level(&p, 2).unwrap_err();
        assert_eq!(err, ReviveError::NoLevels { cost: 2, level: 1 });
    }

    #[test]
    fn reset_exp_keeps_level() {
        let out = resolve_revive(
            &ReviveOption::ResetExp,
            &profile(),
            &ReviveConfig::default(),
        )
        .unwrap();
        assert_eq!(out.exp, 0);
        assert_eq!(out.level, 5);
        assert_eq!(out.current_hp, 100, "session restart restores HP");
    }

    #[test]
    fn restart_scenario_no_penalty() {
        let out = resolve_revive(
            &ReviveOption::RestartScenario,
            &profile(),
            &ReviveConfig::default(),
        )
        .unwrap();
        assert_eq!(out.exp, 400, "no penalty: exp kept");
        assert_eq!(out.level, 5);
        assert_eq!(out.current_hp, 100);
        assert_eq!(out.maseki, 1000);
    }

    #[test]
    fn damage_absorb_leaves_one_hp() {
        let out = resolve_revive(
            &ReviveOption::DamageAbsorb {
                mp_per_turn: 10,
                hp_per_turn: 10,
            },
            &profile(),
            &ReviveConfig::default(),
        )
        .unwrap();
        assert_eq!(out.current_hp, 1);
        assert_eq!(out.current_mp, 100, "MP untouched by the surviving hit");
    }

    #[test]
    fn damage_absorb_turn_converts_while_mp_remain() {
        let cfg = ReviveConfig::default();
        let mut p = profile();
        p.current_hp = 1;
        p.current_mp = 30;
        let t = damage_absorb_turn(&p, &cfg);
        assert_eq!(t.mp_spent, 10);
        assert_eq!(t.hp_gained, 10);
        let after = apply_damage_absorb_turn(&p, &cfg);
        assert_eq!(after.current_mp, 20);
        assert_eq!(after.current_hp, 11);
    }

    #[test]
    fn damage_absorb_turn_stops_when_mp_low() {
        let cfg = ReviveConfig::default();
        let mut p = profile();
        p.current_mp = 5;
        let t = damage_absorb_turn(&p, &cfg);
        assert_eq!(t.mp_spent, 0);
        assert_eq!(t.hp_gained, 0);
        assert_eq!(apply_damage_absorb_turn(&p, &cfg), p);
    }

    #[test]
    fn auto_resurrect_no_penalty() {
        let out = resolve_auto_resurrect(
            &AutoResurrectKind::NoPenalty,
            &profile(),
            &ReviveConfig::default(),
        )
        .unwrap();
        assert_eq!(out.current_hp, 100);
        assert_eq!(out.maseki, 1000);
        assert_eq!(out.bot_lives, 5);
    }

    #[test]
    fn auto_resurrect_costs_bot_lives() {
        let out = resolve_auto_resurrect(
            &AutoResurrectKind::CostBotLives { count: 5 },
            &profile(),
            &ReviveConfig::default(),
        )
        .unwrap();
        assert_eq!(out.bot_lives, 0);
        assert_eq!(out.current_hp, 100);

        let mut poor = profile();
        poor.bot_lives = 3;
        let err = resolve_auto_resurrect(
            &AutoResurrectKind::CostBotLives { count: 5 },
            &poor,
            &ReviveConfig::default(),
        )
        .unwrap_err();
        assert_eq!(
            err,
            ReviveError::NoBotLives {
                required: 5,
                owned: 3
            }
        );
    }

    #[test]
    fn auto_resurrect_mp_to_hp() {
        assert_eq!(mp_to_hp(100, 50), 50);
        assert_eq!(mp_to_hp(30, 50), 15);
        assert_eq!(mp_to_hp(0, 50), 0);

        let out = resolve_auto_resurrect(
            &AutoResurrectKind::MpToHp { percent: 50 },
            &profile(),
            &ReviveConfig::default(),
        )
        .unwrap();
        assert_eq!(out.current_hp, 60, "10 + 50");
        assert_eq!(out.current_mp, 50, "100 - 50");
    }

    #[test]
    fn auto_resurrect_mp_to_hp_requires_mp() {
        let mut p = profile();
        p.current_mp = 1;
        let err = resolve_auto_resurrect(
            &AutoResurrectKind::MpToHp { percent: 50 },
            &p,
            &ReviveConfig::default(),
        )
        .unwrap_err();
        assert_eq!(
            err,
            ReviveError::InsufficientMp {
                required: 1,
                owned: 1
            }
        );
    }

    #[test]
    fn auto_resurrect_pay_maseki_fixed() {
        let out = resolve_auto_resurrect(
            &AutoResurrectKind::PayMaseki { amount: 100 },
            &profile(),
            &ReviveConfig::default(),
        )
        .unwrap();
        assert_eq!(out.maseki, 900);

        let mut poor = profile();
        poor.maseki = 50;
        let err = resolve_auto_resurrect(
            &AutoResurrectKind::PayMaseki { amount: 100 },
            &poor,
            &ReviveConfig::default(),
        )
        .unwrap_err();
        assert_eq!(
            err,
            ReviveError::InsufficientMaseki {
                required: 100,
                owned: 50
            }
        );
    }

    #[test]
    fn maseki_percent_payment_floor() {
        // 10% of 1000 = 100, above the 50 floor
        assert_eq!(maseki_percent_payment(1000, 10, 50), 100);
        // 10% of 200 = 20, floored up to 50
        assert_eq!(maseki_percent_payment(200, 10, 50), 50);
        // floor binds even at 0 owned
        assert_eq!(maseki_percent_payment(0, 10, 50), 50);
    }

    #[test]
    fn auto_resurrect_pay_maseki_percent() {
        let out = resolve_auto_resurrect(
            &AutoResurrectKind::PayMasekiPercent {
                percent: 10,
                min_amount: 50,
            },
            &profile(),
            &ReviveConfig::default(),
        )
        .unwrap();
        assert_eq!(out.maseki, 900, "paid 100 (10% of 1000)");

        let mut low = profile();
        low.maseki = 30;
        let err = resolve_auto_resurrect(
            &AutoResurrectKind::PayMasekiPercent {
                percent: 10,
                min_amount: 50,
            },
            &low,
            &ReviveConfig::default(),
        )
        .unwrap_err();
        assert_eq!(
            err,
            ReviveError::InsufficientMaseki {
                required: 50,
                owned: 30
            }
        );
    }

    #[test]
    fn config_defaults_feed_zeroed_options() {
        let cfg = ReviveConfig::default();
        // levels=0 and count=0 mean "use config default"
        let lose =
            resolve_revive(&ReviveOption::LoseLevel { levels: 0 }, &profile(), &cfg).unwrap();
        assert_eq!(lose.level, 4, "default cost is 1");
        let bots = resolve_auto_resurrect(
            &AutoResurrectKind::CostBotLives { count: 0 },
            &profile(),
            &cfg,
        )
        .unwrap();
        assert_eq!(bots.bot_lives, 0, "default bot-lives cost is 5");
    }

    #[test]
    fn is_affordable_helper() {
        let cfg = ReviveConfig::default();
        assert!(is_affordable(&ReviveOption::ResetExp, &profile(), &cfg));
        let mut p = profile();
        p.level = 0;
        assert!(!is_affordable(
            &ReviveOption::LoseLevel { levels: 1 },
            &p,
            &cfg
        ));
    }

    #[test]
    fn dialog_expires_to_none() {
        let opts = vec![ReviveOption::ResetExp, ReviveOption::RestartScenario];
        let mut dlg = ReviveDialog::new(opts.clone(), 3);
        assert!(!dlg.is_resolved());
        assert!(!dlg.tick());
        assert!(!dlg.tick());
        assert!(dlg.tick(), "expires on the last tick");
        assert!(dlg.is_resolved());
        assert_eq!(dlg.selection, Some(ReviveChoice::None));
        assert_eq!(dlg.options, opts);
    }

    #[test]
    fn dialog_first_choice_wins() {
        let opts = vec![ReviveOption::ResetExp, ReviveOption::RestartScenario];
        let mut dlg = ReviveDialog::new(opts, 60);
        dlg.choose(ReviveChoice::Revive(ReviveOption::RestartScenario));
        dlg.choose(ReviveChoice::Revive(ReviveOption::ResetExp));
        assert_eq!(
            dlg.selection,
            Some(ReviveChoice::Revive(ReviveOption::RestartScenario))
        );
        // ticking after a choice does nothing
        assert!(!dlg.tick());
    }

    #[test]
    fn resolve_revive_via_dialog_selection() {
        let dlg_opts = vec![ReviveOption::ResetExp];
        let mut dlg = ReviveDialog::new(dlg_opts, 60);
        dlg.choose(ReviveChoice::Revive(ReviveOption::ResetExp));
        let profile = profile();
        if let Some(ReviveChoice::Revive(option)) = &dlg.selection {
            let out = resolve_revive(option, &profile, &ReviveConfig::default()).unwrap();
            assert_eq!(out.exp, 0);
            assert_eq!(out.level, 5);
        } else {
            panic!("expected a revive selection");
        }
    }
}
