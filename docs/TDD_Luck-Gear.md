# TDD — Luck Gear (Ring of Luck)

## Status

**Implemented @ `crates/sstd-core/src/items.rs`** — equipable luck items with (a) a drop-luck
bonus and (b) a crit-deflect roll. Seeded by `populate_004`. 87 `sstd-core` tests pass,
clippy-clean.

> **Design note:** Luck gear keeps the dice rolling. A Ring of Luck raises *drop* odds while
> equipped and — separately — rolls to downgrade an inbound *critical* to a normal hit. The
> enemy can wear the same ring; the two luck stats fight over one die.

---

## 1. Design Constraints

1. **Two independent effects per item** — `drop_luck_bonus` (feeds `luck_weighted_rarity`) and
   `crit_deflect_chance` (feeds punish-proof inbound-hit resolution).
2. **Config-driven** — every formula constant lives in `sstd_config.sqlite3` via `populate_004`;
   `LuckGearConfig::default()` is only an unseeded-store fallback.
3. **Bounded deflect** — combined chances capped at `item.deflect_cap`, so stacking rings cannot
   make crits impossible.
4. **Seeded replay** — both rolls run through `RollLog`, so the same seed reproduces the same
   crit/deflect outcome (checksum invariant from `luckbot.rs`).
5. **Pure core, thin bridge** — stage logic in `sstd-core`; GDScript only maps ids/equips.

---

## 2. Config Keys (population `004`)

| Key | Type | Default | Purpose |
|-----|------|---------|---------|
| `item.deflect_cap` | f64 | `0.5` | Max combined crit-deflect probability |
| `item.deflect_per_gear` | f64 | `0.15` | Extra deflect per equipped gear item |
| `item.deflect_bonus_per_luck` | f64 | `0.02` | Extra deflect per point of wearer's luck |

---

## 3. Math

### Drop luck

```
effective_luck = equipLuck + Σ gear.drop_luck_bonus
```

Feed `effective_luck` into `luck_weighted_rarity` — the same weighted table as LuckBot, so a
Ring of Luck stacks naturally with the aura.

### Crit deflect (defender's roll, the "reward")

```
deflect_chance = clamp(
    Σ gear.crit_deflect_chance
  + item.deflect_per_gear * gear_count
  + item.deflect_bonus_per_luck * max(luck,0),
  0, item.deflect_cap)
```

### Inbound hit resolution (two seeded rolls)

```
is_crit  = log.roll()%P   < crit_chance(base_crit, attacker_luck) * P      // 1 if attacker crits
if !is_crit -> Normal
deflected = log.roll()%P  < deflect_chance(gear, defender_luck) * P        // 2 if defender deflects
if deflected -> Deflected else -> Crit
```

`P` is a `100_000`-parts bucket so both chances map to integer roll thresholds. Using `RollLog`
keeps replay bit-exact (page 4.1 of the LuckBot TDD applies).

---

## 4. Public API (`src/`)

Types: `LuckGear { id, drop_luck_bonus, crit_deflect_chance }` (serde, editor UI),
`LuckGearConfig` (Default fallback), `InboundHit::{Normal,Deflected,Crit}`.

Functions (pure):

```rust
pub fn drop_luck_with_gear(luck: i32, gear: &[LuckGear]) -> i32
pub fn deflect_chance(gear, luck, gear) -> f64
pub fn roll_inbound_attack(
    base_crit_chance, attacker_luck, attacker_config: &LuckConfig,
    gear: &[LuckGear], defender_luck, config: &LuckGearConfig, log: &mut RollLog,
) -> InboundHit
```

`ConfigStore::luck_gear_config()` maps `item.*` rows → `LuckGearConfig`.

---

## 5. Edge Cases

| Case | Behaviour |
|------|-----------|
| no gear | deflect = 0; crits pass through |
| deflect capped | never exceeds `item.deflect_cap` |
| attacker crit 0% | `Normal`, only 1 roll drawn (no deflect draw) |
| attacker crit 100% / deflect 100% | always `Deflected` |
| same seed replayed | identical sequence + identical checksum |
| `luck < 0` | clamped to 0 for deflect |

---

## 6. Tests

`#[cfg(test)]` in `items.rs` (`drop_luck_adds_gear_bonus`, `deflect_chance_scales_and_caps`,
`inbound_attack_rolls_deflect`, `inbound_attack_misses_crit_is_normal`,
`roll_is_replayable`) plus `test_luck_gear_keys_seeded`. Run: `cargo test -p sstd-core`.

---

## 7. Open Questions

- **Unique item ids / stacking** — the math sums all equipped gear blindly. A future inventory
  rule ("unique: cannot equip two Rings of Luck") lives in the editor/game layer, not here.
- **Exact roll bucket size** — `ROLL_PERCENT = 100_000` is an implementation detail; fine as-is
  unless precision needs grow.