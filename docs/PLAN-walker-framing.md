# Prototype walker camera framing

TL;DR: Make the Simulator show the avatar, not merely spawn it. Keep the existing GDScript stepper; frame the avatar after screen-entry placement.

Scope: #122 (Simulator ground walker), narrowed by @me to a three-screen flat prototype, extended to 30° hill slopes (up/down) as walkable terrain.

## Evidence and approach

A real window probe (`/tmp/user/1000/opencode/inspect_walker.gd`) measured avatar world (16,992), camera (960,528), viewport (1152,617), and avatar canvas (-368,772.5). The spawn succeeds but the active-screen camera excludes both the avatar and ground. A/D following updates x only, leaving ground below the viewport.

`main.gd::_on_tab_changed` already calls `WorldView.reset_walker_on_screen` after the camera jump. In that method, restrict eligible columns to the selected screen (both bounds), place using `_place_walker_on_col`, then defer framing until layout settles. `follow_avatar` already clamps horizontal centre to map bounds. For entry framing, reuse it and set vertical centre to avatar feet minus one quarter of visible world height: feet appear at three quarters of viewport height. Derive visible height from viewport / camera zoom. Constant-time framing; existing column filtering is linear in ground columns.

Preserve manual pan while idle: no continuous vertical follow. Pure stepper and hole behaviour unchanged. Empty-ground camera fixtures keep the existing screen jump. No races: main-thread scene operations only; test repeated entry, resize and movement.

## Verification

Real-render probe: load the actual main scene, select Simulator, wait for layout, assert avatar canvas lies inside viewport with ground clearance; inject D and verify movement plus continued visibility; save screenshot. Regression: walker, world-view, camera controls, camera controller, parallax, screen-store suites. Regression test uses a constructed in-memory three-screen world, not local world.zip. Mutation: omit deferred framing in a throwaway project copy and require visibility test to fail.
