# Game Design Document — Side-Scroll Tower Defense

> **Status**: Active development — mechanics documented here are implemented or in-progress. Sections marked *TBD* are deferred for later game-design iteration.

---

## 1. Characters (Heroes)

Each hero is a deployable field unit with a unique combat mechanic and passive support identity. Heroes can be enhanced by Apprentice units that extend or reinforce their domain.

### 1.1 Gemama — The Enchantress

**Role**: Weapon buffer / aura support.

**Mechanic — Walk-By Enchantment**:
While Gemama moves (or idles near weapons), she emits a short-range aura. Any weapon inside the aura gains a stacking damage buff per tick. The effect decays when she leaves range.

- AoE radius: *TBD*
- Tick interval: every N frames (configurable)
- Stack ceiling: *TBD* (hard cap or diminishing returns)
- Decay rate: *TBD* (full drain after X seconds away)

**Strategic use**: Positioned along the kill corridor to "paint" weapons before a wave hits. Encourages clustering around her path.

### 1.2 Bunnira

**Role**: *TBD — design deferred.*

- Core mechanic: *TBD*
- Apprentice domain: repair of Bunnira-related assets (*TBD*)

### 1.3 Lira

**Role**: *TBD — design deferred.*

- Core mechanic: *TBD*
- Apprentice domain: repair of Lira-related assets (*TBD*)

### 1.4 Nia

**Role**: *TBD — design deferred.*

- Core mechanic: *TBD*
- Apprentice domain: repair of Nia-related assets (*TBD*)

---

## 2. Apprentices (Repairers)

Apprentices are cheap, spammable support units that auto-repair their hero's domain. Multiple apprentices on the same target divide the workload.

### 2.1 Shared Formula

All apprentice repair calculations currently use a fixed formula driven entirely by `sstd_config.sqlite3` constants. (A per-item script override is possible in a future phase — Phase 5+ / post-1.0 — but is not in scope now. If added, the script language will be TypeScript.) Every apprentice domain exposes exactly three config keys:

| Key Pattern | Purpose |
|-------------|---------|
| `repair.<domain>.cost_m` | Slope multiplier |
| `repair.<domain>.cost_c` | Difficulty-anchored intercept base (default 1) |
| `repair.<domain>.rate` | Base repair per-apprentice per-turn |

Examples: `repair.gemama.cost_m`, `repair.gemama.cost_c`, `repair.gemama.rate`.

#### Formula (standard — linear)

```
damageCost = (currentDamage × cost_m) + (difficulty × cost_c)
repairTurns = ceil(damageCost / (apprenticeCount × rate))
```

- **difficulty** — global multiplier (easy = 0.5, normal = 1.0, hard = 2.0, etc.)
- `cost_c` defaults to 1, meaning the baseline intercept is exactly the difficulty modifier.

#### Formula (optional — logarithmic)

If diminishing returns on damage cost is desired later, swap the Rust function without a schema migration:

```
damageCost = (cost_m × log₂(currentDamage + 1)) + (difficulty × cost_c)
repairTurns = ceil(damageCost / (apprenticeCount × rate))
```

The keys (`cost_m`, `cost_c`, `rate`) stay the same — only the Rust evaluation changes.

- **currentDamage** — amount of damage the target has taken (domain-specific unit: HP, charge, structural integrity)
- **apprenticeCount** — number of apprentices currently repairing the same target
- **rate** — base per-apprentice efficiency (configurable per apprentice type)
- Minimum 1-turn floor regardless of count.

### 2.2 Apprentice Types

| Hero | Apprentice | Repair Domain | Notes |
|------|-----------|---------------|-------|
| Gemama | Weapon Repairer | Weapon damage | Enchanted weapons take extra wear; repairers offset the increased degradation |
| Bunnira | *TBD* | *TBD* | Design deferred |
| Lira | *TBD* | *TBD* | Design deferred |
| Nia | *TBD* | *TBD* | Design deferred |

### 2.3 AI Behavior

- Auto-pick the most-damaged eligible target in range.
- Optional manual targeting (by dragging onto a specific weapon/asset).
- Idle if all targets are at full health.

---

## 3. Gacha / Loot System

### 3.1 Blacksmith Gacha

**Source**: Blacksmith NPC — the player's primary gacha entry point for weapon/tool acquisition.

**Icons**: Anvil (idle / menu state), Forge (active / rolling state). Both rendered as 32×32 pixel art via the standard pixel pipeline (`assets/tiles/` convention with `_32x32.png`).

**Mechanic**: *TBD — design deferred to Phase 5.* Each pull consumes in-game currency and produces a random weapon or enhancement item.

---

## 4. Combat & Physics

(Reserved — see TDD documents for per-second vs per-tick constants.)

### 4.1 Death & Revival

When the player's hero (or a unit) takes a killing blow, the game pauses and a **countdown dialog** opens with a 10-second timer. The player may choose one of five revival options, decline, or let the timer expire — expiry is treated exactly like a deliberate decline.

The five options, for the player's eyes:

| # | Option | Player-facing cost | Notes |
|---|--------|--------------------|-------|
| 1 | Revive now | Lose **1 level** | If the hero has no level to give, it is **permanent death** |
| 2 | Restart session | Reset Exp to 0 | Level kept; self-discouraging exploit (dying right after leveling) |
| 3 | Restart scenario | None | High levels gain nothing from restarting starter maps, so it self-balances |
| 4 | Damage-absorb artifact | Survive at **1 HP**, then convert **10 MP → 10 HP** per turn | Keep some MP or the cushion is gone on the next hit |
| 5 | Auto-resurrect artifacts | Varies: free, 5 bot lives, 50% MP→HP, 100 maseki, or 10% maseki (min 50) | One of five artifact sub-kinds is consumed |

All numbers are config-driven (`resurrect.*` keys); nothing is hard-coded in gameplay code.

### 4.2 LuckBots

Companion bots that make their team luckier around them. Like repairers and guards, a LuckBot
can either **patrol** (roam an area) or **follow** an entity such as a catapult or a guard tower.

- **Radius-based aura** — every ally inside the aura benefits; the higher the bot's luck stat,
  the better the **odds** of favorable rolls for allies inside the radius.
- **Rolls stay in play** — luck keeps the dice rolling and makes good outcomes more likely:
  - **Criticals** — luck raises base crit *chance* (capped, so never 100% runaway).
  - **Item drops** — luck inflates the rarer weights in the drop table, so a roll lands on
    better rarities more often.
- **Neutral by default** — luck `0` means no effect; investing in a LuckBot is a real choice.
- **All tuning is config-driven** (`luck.*` keys) for easy future balance.

### 4.3 Luck Gear (Ring of Luck)

Equipable items that focus luck into one slot:

- **Drop luck buff** — while equipped, adds luck to drop rolls, stacking with the LuckyBot aura.
- **Crit deflect** — when an incoming attack would crit, the ring rolls; on success the crit is
  turned into a normal hit. The **enemy can wear the same ring**, so a critical they roll may be
  flung back — two luck stats contesting one die.
- **Bounded** — stacked deflect chances can never exceed the cap; crits never become impossible.
- **Replay-safe** — every luck roll is seeded, so saves replay identically.
- **All tuning is config-driven** (`item.*` keys via `populate_004`).

---

## 5. Config & Constants

All numeric design values above marked *TBD* are stored in `sstd_config.sqlite3` via the population system (see `config.rs`). Once decided, they will be added as a new `populate_NNN` script.
