# TDD — LuckBot Companion

## Status

**Implemented @ `crates/sstd-core/src/luckbot.rs`** — pure Rust data model for a luck stat that
improves the odds of favorable rolls. Seeded by `populate_003`; 77 `sstd-core` tests pass,
clippy-clean.

> **Design note (2026-08-07):** Luck keeps the dice *in play*. A LuckBot's luck scales the
> probability of a crit and inflates the rarer entries of a weighted drop table — the rolls still
> happen, luck simply makes good outcomes more likely. This is deliberate: rolling for nice drops
> is part of the fun.

---

## 1. Design Constraints

1. **RNG stays, luck stacks the odds** — crits and drops are still rolled by the engine; luck
   multiplies the *base* probability / weights so favorable outcomes land more often.
2. **Config-driven** — every constant lives in `sstd_config.sqlite3` via `populate_003`;
   `LuckConfig::default()` is only a fallback for an unseeded store.
3. **Neutral = no effect** — luck `0` yields `multiplier = 1.0`: crit chance and drop table are
   untouched.
4. **Bounded** — one multiplier applied to the *base* only, and crit probability capped at
   `luck.crit_cap` — no runaway stacking, crits can never exceed 100%.
5. **Pure core, thin bridge** — all math in `sstd-core`; GDScript only moves bots / maps ids.

---

## 2. Config Keys (population `003`)

| Key | Type | Default | Purpose |
|-----|------|---------|---------|
| `luck.default_luck` | i32 | `0` | Neutral luck for entities without a stat |
| `luck.bonus_per_point` | f64 | `0.15` | Multiplier added per luck point |
| `luck.max_luck` | i32 | `5` | Hard ceiling on a luck stat |
| `luck.crit_cap` | f64 | `1.0` | Cap on resulting crit probability |
| `luck.aura_radius` | f32 | `3` | Aura radius (tiles) |

---

## 3. Math

### Luck multiplier (the single scaling factor)

```
luck_multiplier = 1.0 + clamp(luck, 0, max_luck) * bonus_per_point
```

Neutral `0` → `1.0`. `luck = 1` @ `0.15` → `1.15`.

### Crit roll

```
crit_chance = clamp(base_crit * luck_multiplier, 0, crit_cap)
```

The engine rolls against `crit_chance`; luck only raises the odds. Base-only scaling + cap keeps
it from ever reaching 100% runaway.

### Rarity roll (weighted table)

Each tier has a base weight; all weights get the same `luck_multiplier`:

```
effective_weight[i] = round(base_weight[i] * luck_multiplier)
total               = Σ effective_weight
roll = R % total; WeightedTable::pick(roll) => tier index
```

Higher luck inflates rarer tiers' share of `total` → a single die roll lands up-table more often.

| Tier | Base weight |
|------|-------------|
| common | 60 |
| uncommon | 28 |
| rare | 9 |
| epic | 2 |
| legendary | 1 |

---

## 4. Public API (`src/`)

- `Bot { id, move_mode: MoveMode, patrol_x/y, follow_target, luck }`
  builders: `.new`, `.with_luck`, `.patrolling(x,y)`, `.following(target)`
- `MoveMode` — `Patrol | Follow` (serde, editor UI)
- `LuckConfig` — `Default` fallback only
- `RarityTier`, `DEFAULT_RARITIES`, `WeightedTable<'a>` (`weights`, `total`, `pick(roll)`)
- `clamp_luck`, `luck_multiplier`
- `crit_chance(base, luck, cfg) -> f64`
- `luck_weighted_rarity(tiers, luck, cfg) -> WeightedTable<'a>`
- `distance`, `within_aura(bot, x, y, cfg) -> bool`
- `seeded_roll(seed, counter) -> u64` — splitmix64 PRNG, dependency-free
- `RollLog::new(seed)` / `.roll()` / `.counter()` / `.checksum()` — replay guard

All pure. `ConfigStore::luck_config()` maps `luck.*` rows → `LuckConfig`.

## 4.1 Replay (Same Seed ⇒ Same Outcome)

`RollLog` makes replay **bit-exact and self-verifying**:

- **Persist the seed** (a single `u64`) — not the dropped tiers. Rolls are derived.
- Each draw = `seeded_roll(seed, counter)`, deterministically folded into
  `checksum()` (a rotated sum). Feed the same seed and issue the same number of
  `.roll()` calls → identical rolls, identical checksum.
- **Checksum is the CI invariant** — on replay, if `checksum` diverges from the
  saved value, a call-order change, missing roll, or PRNG/config swap is the
  culprit. Guided tests cover: same seed ⇒ same stream, replay of a known count
  preserves the checksum, distinct seeds diverge, a missing roll breaks it.

---

## 5. Edge Cases

| Case | Behaviour |
|------|-----------|
| `luck < 0` | clamped to `0` (never anti-luck) |
| `luck > max_luck` | clamped to `max_luck` (never runaway) |
| base crit negative | clamped to `0` |
| `crit_cap < 0` | clamped to `0` (defensive) |
| empty rarity table | `total = 0`, `pick` → `None` |
| roll beyond total | `pick` → `None` |
| neutral luck | multiplier `1.0` — identical to no luck |

---

## 6. Tests

`#[cfg(test)]` in `luckbot.rs` plus `test_luck_keys_seeded` and `test_luck_config_loader`.
Run: `cargo test -p sstd-core`.

---

## 7. Open Questions

- **Drop-table overrides** — weights are code defaults. A config key (e.g. comma-separated
  `drop.weights = "60,28,9,2,1"`) may replace the const table later, keeping tuning in DB.
- **Follow anchor** — `move_anchor()` returns the patrol point; `Follow` relies on the engine
  updating the anchor as the target moves. Confirm contract with the simulator.