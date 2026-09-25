# Editor Tests

Headless GDScript regression runner for the World Editor. Covers the `ScreenStore` registry and the screen operations of the Map Editor and Placement Editor.

## Running

Requires the Godot 4 binary (4.4.x stable).

```bash
godot4 --headless --path .. --script res://tests/test_screen_store.gd
```

The runner is a `SceneTree` script (it does **not** call `quit()` on load), so the process exits itself when the suite finishes.

### Exit codes

**The exit code is authoritative.** A suite runner calls `quit()` on its final line with
`0` on success and non-zero on failure, in this exact form:

```gdscript
quit(0 if _failures == 0 else 1)
```

Gate on the exit code, not on the printed `failures=0` line — the line is a
human-readable summary and a suite can print it while its exit code says otherwise.
`test_image_to_map.gd` and `test_terrain_brush.gd` shipped an inverted ternary
(`quit(1 if failures == 0 else 2)`) that made a *passing* run exit non-zero; it went
unnoticed because a failing run also exits non-zero, which satisfies an exit-code check
for the wrong reason. See [#80](https://github.com/HidekiAI/sidescroll-towerdefense/issues/80).

Probe and capture scripts under this directory are not suites and use their own
conventions (`quit(0)` when the capture succeeded, `quit(1)` on a hard abort).

## Coverage

- **ScreenStore** — register/occupy/id lookup, move (incl. rejection onto an occupied cell), remove, `next_id`, `next_free_position`, bounds, `save_world`/`load_world` round-trip.
- **Map Editor screen ops** — add screens, switch to an empty screen, tile preserved across switches, unregistered id → unplaced + empty canvas, clone (registry + file written), move via store, minimap dialog setup.
- **Placement Editor screen ops** — entity placed, placements cleared on switch, preserved on switch-back, `_apply_screen` (nested `tile_data` + flattened keys + `placed_entities`).

## Conventions

- Each test creates a fresh `ScreenStore` with `dir = /tmp` (see `_make_store`), so runs never touch committed data.
- Test cases assert with `check(cond, label)`; failures accumulate and print under a `--- <suite> ---` header.
- A test exercising screen switching must use a **screen id that is not registered** (the store has real screens at `(0,0)`/`(1,0)`), otherwise the "empty canvas" assertion is invalid.

## Related

- Screen registry design: `scripts/screen_store.gd`
- File schema it validates against: [TDD_Saved-World](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TDD_Saved-World)
