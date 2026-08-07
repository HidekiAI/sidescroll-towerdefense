# TDD — Death / Revive System

## Status

**Implemented @ `crates/sstd-core/src/resurrection.rs`** — Rust data model, pure resolution logic, and unit tests are landed and green (74 `sstd-core` tests pass). `ConfigStore::revive_config()` loader + population `002` seed all tuning keys. The GDScript countdown modal (scene + script) is **deferred** — see Open Questions.

---

## 1. Design Constraints

1. **Pure core, thin bridge** — all math lives in `sstd-core`, as pure functions over a `PlayerProfile + ReviveConfig`, returning `Result<PlayerProfile, ReviveError>`. The editor/game GDScript only drives the countdown dialog and calls `resolve_revive`. No logic in GDScript.
2. **Permanent death is the only hard failure** — `LoseLevel` with `cost > current_level` (including level `0`) returns `ReviveError::NoLevels`, which the caller treats as permanent death. **No negative levels.**
3. **Countdown modal is always quittable** — the player may select an option, decline, or let the timer expire; expiry is treated identically to a manual quit (decline / accept-death). The countdown forces a decision under time pressure but never a *wrong* decision.
4. **First selection wins** — `ReviveDialog` is single-shot; once a choice or expiry resolves it, later input is ignored.
5. **Anti-cheat is a design property, not a patch** — two of the five options are inherently self-balancing (see §4).

---

## 2. Config Keys (population `002`)

All rows live in `config` and are seeded by `populate_002` (id `"002"`).

| Key | Type | Default | Purpose |
|-----|------|---------|---------|
| `resurrect.lose_level_cost` | u32 | `1` | Levels dropped by Option 1 |
| `resurrect.damage_absorb_leave_hp` | i32 | `1` | HP left after the killing hit (Option 4) |
| `resurrect.damage_absorb_mp_per_turn` | u32 | `10` | MP spent per damage-absorb tick |
| `resurrect.damage_absorb_hp_per_turn` | u32 | `10` | HP gained per damage-absorb tick |
| `resurrect.auto_resurrect_bot_lives` | u32 | `5` | Bot-lives cost (Option 5a) |
| `resurrect.auto_resurrect_mp_to_hp_percent` | u32 | `50` | MP→HP transfer % (Option 5b) |
| `resurrect.auto_resurrect_pay_maseki_amount` | u64 | `100` | Flat maseki cost (Option 5c) |
| `resurrect.auto_resurrect_pay_maseki_percent` | u32 | `10` | Maseki % cost (Option 5d) |
| `resurrect.auto_resurrect_pay_maseki_min` | u64 | `50` | Floor maseki for % cost (Option 5d) |
| `resurrect.dialog_countdown_secs` | u32 | `10` | Countdown seconds |

---

## 3. The Five Revival Options

Countdown modal (`ReviveDialog`) presents a `Vec<ReviveOption>`; the player picks one,
declines, or times out (`ReviveOutcome::{Revived, Declined}`).

### Option 1 — Lose levels (revive immediately)
`level -= cost`. Cheap at high level, **permanent death** if `level < cost`.

### Option 2 — Restart session (reset Exp)
`exp = 0`, full HP/MP restore, level unchanged. *Inherent anti-cheat:* dying right after
leveling nearly costs nothing → the exploit is symmetrical and unfun for the resetting player,
so it self-discourages.

### Option 3 — Restart scenario, no penalty
Full restore, no cost. *Inherently self-balancing:* high-level players restarting starter
scenarios gain negligible Exp, so the "free" exploit disappears at level.

### Option 4 — Damage-absorb artifact
Survive the killing hit at `damage_absorb_leave_hp`. Then, each tick while MP remain,
convert `damage_absorb_mp_per_turn` MP → `damage_absorb_hp_per_turn` HP
(`damage_absorb_turn()` / `apply_damage_absorb_turn()`). Resource-bounded: run out of MP and
the 1-HP cushion is burned next hit.

### Option 5 — Auto-resurrect artifacts (five sub-kinds)
| Sub-kind | Cost | Formula |
|----------|------|---------|
| `NoPenalty` | none | full restore |
| `CostBotLives { count }` | bot lives | default `auto_resurrect_bot_lives` when `count==0`; error `NoBotLives` if insufficient |
| `MpToHp { percent }` | MP | `hp = mp × pct/100`, MP deducted accordingly; error `InsufficientMp` if result ≤ 0 |
| `PayMaseki { amount }` | maseki | flat; default from config when `amount==0` |
| `PayMasekiPercent { pct, min }` | maseki | `max(⌊owned×pct/100⌋, min)`; defaults from config when 0 |

Any option with a `0` cost field falls back to the corresponding config default, so a dialog
can be built generically and the exact tuning stays in the DB.

---

## 4. Public API (`sstd-core`)

Types:

- `PlayerProfile` — level, exp, HP, MP, maseki, bot_lives (plain struct, no serde)
- `AutoResurrectKind` / `ReviveOption` — serde + `JsonSchema` (travel through the editor UI)
- `ReviveConfig` — defaults; load from `ConfigStore` at start
- `ReviveError` — `NoLevels`, `NoBotLives`, `InsufficientMp`, `InsufficientMaseki`
- `ReviveOutcome` — `Revived { profile, option }` | `Declined { reason }`
- `DeclineReason` — `PlayerDeclined` | `Expired` | `Error(ReviveError)`
- `ReviveChoice` — `Revive(option)` | `None`
- `ReviveDialog` — pure countdown state `new/ tick/ choose/ is_resolved (option, deadline_ticks, remaining_ticks, selection)`

### Functions (all pure, `Result`-based)

```rust
pub fn resolve_revive(option, profile, config) -> Result<PlayerProfile, ReviveError>
pub fn revive_lose_level(profile, cost)                         -> Result<..>
pub fn resolve_auto_resurrect(kind, profile, config)            -> Result<..>
pub fn hp = mp_to_hp(mp, percent)                                -> i32          // ⌊mp×p/100⌋
pub fn maseki_percent_payment(owned, percent, min)              -> u64          // max(⌊owned×p/100⌋, min)
pub fn damage_absorb_turn(profile, config)                      -> DamageAbsorbTurn // 0/0 if no MP
pub fn apply_damage_absorb_turn(profile, config)                -> PlayerProfile
pub fn is_affordable(option, profile, config)                   -> bool         // resolve().is_ok()
```

---

## 5. Edge Cases

| Case | Behaviour |
|------|-----------|
| level `0`, Option 1 chosen | `NoLevels` → **permanent death** |
| `cost == 0` | dialog used, engine substitutes config default |
| MP < one tick's cost | tick yields 0/0 (no spend, no heal) |
| MP→HP would give ≤ 0 HP | `InsufficientMp` (can't resurrect on 0 MP) |
| bot_lives / maseki insufficient | matching `ReviveError` |
| HP over max after regeneration | clamped to `max_hp` |
| expire vs manual decline | both → `Declined { Expired | PlayerDeclined }` |
| double `choose()` | first wins (dialog ignores later input) |

---

## 6. Configuration Loading

```rust
impl ConfigStore {
    pub fn revive_config(&self) -> SstdResult<ReviveConfig> {
        // implemented in config.rs; maps each resurrect.* key, parsing or
        // falling back to ReviveConfig::default().
    }
}
```

(`populate_002` seeds the keys; the loader maps rows onto `ReviveConfig`. Backed by
`test_revive_config_loader`.)

---

## 7. Tests

`#[cfg(test)]` in `resurrection.rs` (~19) plus `config::tests::test_resurrect_keys_seeded`
and the updated `test_config_store_in_memory` (22 keys) and `test_schema_version` (0.0.2).

Run:

```sh
cargo test -p sstd-core
```

---

## 8. Open Questions

- **GDScript countdown modal (deferred)** — the scene (`resurrection_dialog.tscn`) + script
  (`resurrection_dialog.gd`) that drives `ReviveDialog` and calls `resolve_revive` are not yet
  authored; the core types and loader are ready for it.
- Should the damage-absorb passive clear itself when HP reaches max (stop burning MP once full)?
  *Proposal: yes — stop early rather than waste MP; currently it runs while MP remain.*
- Player how keep `bot` artefact ownership — separate inventory table, or a derived counter on the profile? Currently the profile just holds `bot_lives`.