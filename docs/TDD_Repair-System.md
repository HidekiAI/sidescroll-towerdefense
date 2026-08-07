# TDD — Repair System

## Status

**Implemented-in-principle** — design approved, awaiting per-character domain details for Bunnira/Lira/Nia. Gemama (weapon) repair values are ready for population scripting.

---

## 1. Design Constraints

1. **Constant formula as primary path** — all calculations use a fixed formula with config constants from `sstd_config.sqlite3` (`cost_m`, `cost_c`, `rate`). No Lua, DSL, or eval on the critical path.
2. **Per-item script is deferred** — a future phase (Phase 5+ / post-1.0) could add an optional `repair_script` column to the config table for items whose formula overrides the global one. That is explicitly **not** in scope now. The script language would be **TypeScript**.
3. **Idempotent by design** — the formula is pure function of `(damage, count, cost_m, cost_c, rate, difficulty)`. No state, no side effects.
4. **Config-driven** — every numeric parameter is a row in `config` table, seeded via the population system.

---

## 2. Config Keys

Each apprentice domain registers exactly three keys (pattern `repair.<domain>.<key>`):

| Key | Type | Default | Purpose |
|-----|------|---------|---------|
| `repair.<domain>.cost_m` | f64 | `*TBD*` | Slope multiplier |
| `repair.<domain>.cost_c` | f64 | `1` | Difficulty-anchored intercept base; final intercept = `difficulty × cost_c` |
| `repair.<domain>.rate` | f64 | `*TBD*` | Base repair per apprentice per turn |

### Domains

| Domain | Hero | Apprentice | Target Unit |
|--------|------|------------|-------------|
| `gemama` | Gemama | Weapon Repairer | Weapon current damage |

Bunnira, Lira, Nia domains are *TBD — design deferred*.

---

## 3. Formula

### 3.1 Linear (default)

```
damageCost = (currentDamage × cost_m) + (difficulty × cost_c)
repairTurns = ceil(damageCost / (apprenticeCount × rate))
```

- **difficulty** — global multiplier (easy = 0.5, normal = 1.0, hard = 2.0)
- `cost_c` defaults to 1, so the baseline intercept is exactly the current difficulty.

### 3.2 Logarithmic (opt-in by swapping Rust evaluation)

```
damageCost = (cost_m × log₂(currentDamage + 1)) + (difficulty × cost_c)
repairTurns = ceil(damageCost / (apprenticeCount × rate))
```

No schema migration needed for the swap — the three config keys are identical; only the Rust `fn repair_turns(...)` changes.

### 3.3 Edge Cases

| Case | Behaviour |
|------|-----------|
| `currentDamage <= 0` | Return 0 (nothing to repair) |
| `apprenticeCount == 0` | Return `i32::MAX` or skip (no repairers assigned) |
| `rate <= 0` | Panic / clamp to 1 (invalid config) |
| `repairTurns == 0` | Clamp to 1 (every repair takes at least 1 turn) |

---

## 4. Rust API (proposed)

```rust
// In sstd-core, domain::repair module (or config module)

pub struct RepairParams {
    pub cost_m: f64,
    pub cost_c: f64,
    pub rate: f64,
}

pub struct RepairInput {
    pub current_damage: f64,
    pub apprentice_count: u32,
    pub difficulty: f64,
}

pub enum RepairFormula {
    Linear,
    Logarithmic,
}

pub fn repair_turns(
    input: RepairInput,
    params: &RepairParams,
    formula: RepairFormula,
) -> u32 {
    let intercept = input.difficulty * params.cost_c;
    let damage_cost = match formula {
        RepairFormula::Linear => input.current_damage * params.cost_m + intercept,
        RepairFormula::Logarithmic => {
            params.cost_m * (input.current_damage + 1.0).log2() + intercept
        }
    };
    let turns = (damage_cost / (input.apprentice_count as f64 * params.rate)).ceil() as u32;
    turns.max(1)
}
```

Load `RepairParams` from ConfigStore at game start:

```rust
impl ConfigStore {
    pub fn repair_params(&self, domain: &str) -> SstdResult<RepairParams> {
        Ok(RepairParams {
            cost_m: self.get_str(&format!("repair.{}.cost_m", domain))?.parse().unwrap_or(1.0),
            cost_c: self.get_str(&format!("repair.{}.cost_c", domain))?.parse().unwrap_or(1.0),
            rate:   self.get_str(&format!("repair.{}.rate",   domain))?.parse().unwrap_or(1.0),
        })
    }
}
```

---

## 5. Population Script

When design values for `repair.gemama.cost_m` and `repair.gemama.rate` are finalized, create `populate_004` (populations `002` (death/revive) and `003` (LuckBot) are taken; repair lands in `004`):

```rust
fn populate_004(conn: &Connection) -> SstdResult<()> {
    let entries: [(&str, &str); 3] = [
        ("repair.gemama.cost_m", "<TBD>"),
        ("repair.gemama.cost_c", "1"),
        ("repair.gemama.rate",   "<TBD>"),
    ];
    for (key, value) in &entries {
        conn.execute(
            "INSERT OR IGNORE INTO config (key, default_value) VALUES (?1, ?2)",
            params![key, value],
        )?;
    }
    Ok(())
}
```

Register in the `POPULATIONS` array in `config.rs`:

```rust
const POPULATIONS: &[Population] = &[
    Population { id: "001", description: "Core config: grid, time, entity limits", func: populate_001 },
    Population { id: "002", description: "Death/revive system", func: populate_002 },
    Population { id: "003", description: "LuckBot companion: luck stat, aura radius, crit/rarity tuning", func: populate_003 },
    Population { id: "004", description: "Repair system: Gemama weapon repair constants", func: populate_004 },
];
```

---

## 6. Open Questions

- Should `repair_turns` expose the original `damageCost` to the UI (so the player sees "72 repair-damage worth of work, 3 repairers at rate 5 = 5 turns")? *Proposal: yes — return a struct with both `damage_cost` and `turns`.*
