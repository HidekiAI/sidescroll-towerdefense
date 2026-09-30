# slice-layers (RETIRED as the normal path to parallax layers)

**Status:** kept as a one-off recovery tool. Not the way SSTD produces backdrop layers.

Design of record: [TDD_Parallax-Background §4.1.1](https://github.com/HidekiAI/sidescroll-towerdefense/wiki/TechnicalDesign/TDD_Parallax-Background).
The normal path is [`tools/gen-backdrop`](../gen-backdrop) (planned), which generates
layers in-repo with a wrapping noise lattice.

## What it does

Recovers per-depth RGBA parallax layer PNGs from a rendered camera-pan **video**, by
**flow, not ML**: each depth plane moves `delta = camera_delta * factor`, so far planes
barely move and near planes move a lot, and measured horizontal displacement encodes
depth by construction.

1. Coarse block match (8x8 SAD, horizontal search) over two frames ~1 s apart, with a
   reliability median-fill and border smear for featureless regions.
2. Cluster the displacement histogram into `--bands` (default 3) via 1-D k-means with 3
   deterministic inits, keeping the lowest-SSE one, plus a robust p5..p95 single-plane
   reject.
3. Compose full-res RGBA per band with feathered alpha seams -> `layer_N.png`
   far-to-near, plus a `displacement.png` diagnostic.

```sh
slice-layers <frame_a> <frame_b> [--bands N] [--out DIR] [--scale S]
slice-layers --frame <f> --depth <depth_png> [--bands N] [--out DIR]
```

## Why it is retired

Its only inputs are local-only and gitignored (user directive 2026-09-13):
`assets/samples/preview-with-parallax*.mp4` and the 1 fps frame directory
`assets/samples/preview-with-parallax-extended/`. A fresh clone **cannot** re-run this
tool, so its output is an artifact whose provenance a new contributor cannot reproduce
or check. The 3 layers it produced are superseded by the 5-layer generated stack.

## Reproducing the original run

Only possible on the machine that holds the MP4s:

```sh
cargo run --release -p slice-layers -- <frame_a> <frame_b> --out assets/backdrop_layers
```

with frame pairs extracted to `/tmp` at 1 fps. The committed
`assets/backdrop_layers/*.png` were the derived artifact of record; they are removed by
the #68 restack (see the plan at `docs/PLAN-2026-09-30-parallax-tutorial-stack.md`).

## Tests

4 unit tests in-crate. The algorithm choice and its rationale are documented in the
`//!` header of `src/main.rs`, including why an onnxruntime depth model was rejected
(native dependency plus a model download, for a proxy the video already provides).
