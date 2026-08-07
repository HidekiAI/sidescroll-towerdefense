# TDD — LuckBot Companion

## Status

**Implemented @ `crates/sstd-core/src/luckbot.rs`** — pure Rust data model: deterministic,
bounded luck modifier (no RNG), crit cadence, guaranteed rarity floor, and aura geometry.
Seeded by `populate_003`. 74 `sstd-core` tests pass, clippy-clean.

> **Design note:** SSTD is strategic / no-RNG. Luck is a **predictable bounded modifier** —
> `final_damage × (1 + %LUCK)`, capped. Luck never rolls dice; it guarantees a stronger outcome
> up to a hard cap.

---

## 1. Design Constraints

1. **Deterministic, never random** — luck applies a guaranteed, bounded multiplier. No dice.
2. **Config-driven** — every constant lives in `sstd_config.sqlite3` via `populate_003`;
   `LuckConfig::default()` is only a fallback for an unseeded store.
3. **Neutral = no effect** — luck `0` gives a `+0%` bonus: `final == base` exactly.
4. **Bounded** — the total boost is capped (`luck.bonus_cap`, default `+25%`), so luck feels
   strong but never overpowering.
5. **Pure core, thin bridge** — all math in `sstd-core`; GDScript only moves bots / maps ids.

---

## 2. Config Keys (population `003`)

| Key | Type | Default | Purpose |
|-----|------|---------|---------|
| `luck.default_luck` | i32 | `0` | Neutral luck for entities without a stat |
| `luck.bonus_per_point` | f64 | `0.15` | % bonus added per luck point |
| `luck.max_luck` | i32 | `3` | Hard ceiling on a luck stat |
| `luck.bonus_cap` | f64 | `0.25` | Hard ceiling on the total boost |
| `luck.aura_radius` | f32 | `3` | Aura radius (tiles) |

---

## 3. Math

### Luck bonus (the single source of the boost)

```
bonus_fraction = min(luck * bonus_per_point, bonus_cap)     // luck clamped to [0, max_luck]
final_damage   = base_damage * (1 + bonus_fraction)
```

Neutral `0` → `bonus = 0` → `×1.0`. `luck = 1` @ `0.15` → `+15%`.

### Crit (deterministic cadence, not a chance)

`crit_interval(luck) -> Option<u32>`:

- `bonus = 0` → `None` (never crits).
- Otherwise → `Some(ceil(1 / bonus))` — crit every N-th attack.
- Max luck (cap reached) → `Some(1)` — every attack crits.

No RNG: the cadence is purely a function of the (config-bounded) luck value.

### Rarity (deterministic floor, not weighted rolls)

`rarity_floor(luck, tier_count)`: luck raises the **minimum** tier a drop can be.
Linear and bounded — max luck guarantees the top tier.

```
floor = level * (tier_count - 1) / max(max_luck, 1)
```

---

## 4. Public API (`src/`)

- `Bot { id, move_mode: MoveMode, patrol_x/y, follow_target, luck }`
  builders: `.new`, `.with_luck`, `.patrolling(x,y)`, `.following(target)`
- `MoveMode` — `Patrol | Follow` (serde, editor UI)
- `LuckConfig` — `Default` fallback only
- `clamp_luck`, `luck_bonus_fraction`, `damage_multiplier`
- `crit_interval(luck) -> Option<u32>`
- `rarity_floor(luck, tier_count) -> usize`
- `distance`, `within_aura(bot, x, y, cfg) -> bool`

All pure; `loadConfigStore::luck_config()` in `config.rs` maps rows → `LuckConfig`.

---

## 5. Edge Cases

| Case | Behaviour |
|------|-----------|
| `luck < 0` | clamped to `0` (never negative) |
| `luck > max_luck` | clamped to `max_luck` (never unbounded) |
| `bonus_cap` invalid ≤ 0 | clamped to `[0,1]` defensively |
| neutral luck | `×1.0` damage; no crit; common-only drops |
| max luck | `+25%`, crit every hit, top-tier drops |

---

## 6. Tests

`#[cfg(test)]` in `luckbot.rs` plus `test_luck_keys_seeded` and `test_luck_config_loader`.
Run: `cargo test -p sstd-core`.

---

## 7. Open Questions

- **Drop table** — currently a generic `rarity_floor`; a concrete drop-table keyed by scenario
  is deferred. Weights would be config-driven (`drop.*` keys) if added.
- **Follow anchor** — `move_anchor()` returns the patrol point; `Follow` relies on the engine
  moving the patrol anchor as the target moves. Confirm this contract with the simulator.