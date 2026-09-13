//! slice-layers: recover per-depth RGBA layer PNGs from a rendered parallax pan video.
//!
//! AMBIGUITY RESOLVED (issue #74, decided preplan 2026-09-13): the only surviving
//! artifact of a parallax backdrop was a rendered MP4; no per-layer source art and no
//! recoverable flat background image existed. The depth proxy is MEASURED HORIZONTAL
//! DISPLACEMENT between two frames ~1s apart, NOT a machine-learning depth model.
//! Rationale: the video is a constant-ish camera pan, so each depth plane moves
//! `delta = camera_delta * factor`; far planes barely move, near planes move a lot.
//! Displacement therefore encodes depth by construction, needs zero external model
//! (the `image` crate only), and is ground truth for THIS video. An onnxruntime depth
//! model was considered and rejected: native dependency + model download for a proxy
//! the video already provides. This is an asset-production tool only; the #68 runtime
//! renderer (Parallax2D wiring) is a separate open issue.
//!
//! Usage:
//!   slice-layers <frame_a> <frame_b> [--bands N] [--out DIR] [--scale S]
//!   slice-layers --frame <f> --depth <depth_png> [--bands N] [--out DIR]
//!
//! Outputs: layer_0..N-1.png (far->near, RGBA, alpha = feathered holes),
//!          displacement.png (diagnostic, magnitude visualized 8-bit).
//! Every run writes the input paths, dims, per-band delta stats, band pixel counts
//! and output paths to stdout so an incident is replayable from the log alone.

use image::{DynamicImage, GrayImage, ImageBuffer, Luma, RgbaImage};
use std::path::PathBuf;

const MAX_BANDS: usize = 7;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match run(&args) {
        Ok(out_dir) => println!("slice-layers: done -> {}", out_dir.display()),
        Err(e) => {
            eprintln!("slice-layers: error: {}", e);
            std::process::exit(2);
        }
    }
}

fn run(args: &[String]) -> Result<PathBuf, String> {
    let mut bands: usize = 3;
    let mut scale_hint: u32 = 2;
    let mut out_dir: PathBuf = PathBuf::from("assets/backdrop_layers");
    let mut frame_a: Option<String> = None;
    let mut frame_b: Option<String> = None;
    let mut depth: Option<String> = None;

    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--bands" => {
                i += 1;
                bands = parse_bands(&args[i])?;
            }
            "--scale" => {
                i += 1;
                scale_hint = args[i]
                    .parse::<u32>()
                    .map_err(|_| format!("invalid --scale '{}'", args[i]))?;
                if scale_hint == 0 || scale_hint > 4 {
                    return Err(format!("scale {} out of range (1..4)", scale_hint));
                }
            }
            "--out" => {
                i += 1;
                out_dir = PathBuf::from(&args[i]);
            }
            "--frame" => {
                i += 1;
                frame_a = Some(args[i].clone());
            }
            "--depth" => {
                i += 1;
                depth = Some(args[i].clone());
            }
            other => {
                if frame_a.is_none() {
                    frame_a = Some(other.to_string());
                } else if frame_b.is_none() {
                    frame_b = Some(other.to_string());
                } else {
                    return Err(format!("unexpected positional '{}'", other));
                }
            }
        }
        i += 1;
    }

    let frame_a = frame_a.ok_or_else(|| "--frame/positional frame_a is required".to_string())?;
    let img_a = load_img(&frame_a)?;
    let (gw, gh) = (img_a.width(), img_a.height());
    println!(
        "slice-layers: frame_a={} dims={}x{} bands={} out={}",
        frame_a,
        gw,
        gh,
        bands,
        out_dir.display()
    );

    let displacement: GrayImage = match &depth {
        Some(d) => {
            println!("slice-layers: mode=depth depth_map={}", d);
            let dmap = load_gray(d)?;
            if dmap.width() != gw || dmap.height() != gh {
                return Err(format!(
                    "depth map {}x{} does not match frame {}x{}",
                    dmap.width(),
                    dmap.height(),
                    gw,
                    gh
                ));
            }
            dmap
        }
        None => {
            let fb = frame_b
                .ok_or_else(|| "second frame required unless --depth is given".to_string())?;
            let img_b = load_img(&fb)?;
            if img_b.width() != gw || img_b.height() != gh {
                return Err(format!(
                    "frames differ: a {}x{} vs b {}x{}",
                    gw,
                    gh,
                    img_b.width(),
                    img_b.height()
                ));
            }
            println!("slice-layers: frame_b={} scale={}", fb, scale_hint);
            horizontal_flow(&img_a, &img_b, scale_hint)?
        }
    };

    let (labels, centers) = cluster_displacements(&displacement, bands)?;
    let dw = displacement.width();
    let dh = displacement.height();
    let mut total = 0u64;
    for (b, c) in centers.iter().enumerate() {
        let count = labels.iter().filter(|&&l| l == b as u8).count();
        total += count as u64;
        println!(
            "slice-layers: band={} pixels={} mean_delta={:.1}px",
            b, count, c
        );
    }
    println!(
        "slice-layers: total_labeled={} (displacement {}x{}={})",
        total,
        dw,
        dh,
        dw as u64 * dh as u64
    );

    let near_max = centers[bands - 1].max(0.05);
    let seed: Vec<f32> = centers
        .iter()
        .map(|&c| (0.9 * c / near_max).clamp(0.02, 0.9))
        .collect();
    println!(
        "slice-layers: factor_seed(far->near): [{}]",
        seed.iter()
            .map(|s| format!("{:.2}", s))
            .collect::<Vec<_>>()
            .join(", ")
    );

    std::fs::create_dir_all(&out_dir).map_err(|e| format!("mkdir {}: {}", out_dir.display(), e))?;

    // Labels live at displacement resolution; nearest-upsample to full frame so
    // every frame pixel gets a band (layer alpha must cover the whole screen).
    let full_labels = if dw == gw && dh == gh {
        labels
    } else {
        nearest_upsample(&labels, dw, dh, gw, gh)
    };

    let layers = compose_layers(&img_a, &full_labels, bands)?;
    for (b, lay) in layers.iter().enumerate() {
        let path = out_dir.join(format!("layer_{}.png", b));
        lay.save(&path)
            .map_err(|e| format!("save {}: {}", path.display(), e))?;
        println!("slice-layers: wrote {}", path.display());
    }
    let dpath = out_dir.join("displacement.png");
    displacement
        .save(&dpath)
        .map_err(|e| format!("save {}: {}", dpath.display(), e))?;
    println!("slice-layers: wrote {}", dpath.display());

    Ok(out_dir)
}

fn parse_bands(raw: &str) -> Result<usize, String> {
    let n = raw
        .parse::<usize>()
        .map_err(|_| format!("invalid --bands '{}'", raw))?;
    if !(2..=MAX_BANDS).contains(&n) {
        return Err(format!("--bands must be 2..={}", MAX_BANDS));
    }
    Ok(n)
}

fn load_img(path: &str) -> Result<DynamicImage, String> {
    image::ImageReader::open(path)
        .map_err(|e| format!("open {}: {}", path, e))?
        .decode()
        .map_err(|e| format!("decode {}: {}", path, e))
}

fn load_gray(path: &str) -> Result<GrayImage, String> {
    Ok(load_img(path)?.to_luma8())
}

/// Horizontal block-matching displacement magnitude (px, full-resolution), computed
/// on frames downscaled by `scale` (1 = native). Output image shares flow resolution.
fn horizontal_flow(a: &DynamicImage, b: &DynamicImage, scale: u32) -> Result<GrayImage, String> {
    let sw = a.width() / scale;
    let sh = a.height() / scale;
    let ga: ImageBuffer<Luma<u8>, Vec<u8>> =
        image::imageops::resize(&a.to_luma8(), sw, sh, image::imageops::FilterType::Triangle);
    let gb: ImageBuffer<Luma<u8>, Vec<u8>> =
        image::imageops::resize(&b.to_luma8(), sw, sh, image::imageops::FilterType::Triangle);

    let win = 8u32;
    let max_shift = (sw / 4) as i32;
    let win_half = (win / 2) as i32;
    let offsets: Vec<i32> = (-max_shift..=max_shift).step_by(2).collect();

    let mut flow = GrayImage::new(sw, sh);
    let mut sad_min = vec![f64::MAX; (sw * sh) as usize];

    let y_lo = win_half.max(0);
    let y_hi = (sh as i32 - win_half).min(sh as i32);
    let x_lo = win_half.max(0);
    let x_hi = (sw as i32 - win_half).min(sw as i32);

    let mut n_visited = 0u64;
    let mut sum_min = 0f64;
    for y in y_lo..y_hi {
        for x in x_lo..x_hi {
            let mut best = f64::INFINITY;
            let mut best_off = 0i32;
            for &off in &offsets {
                let s = sad_window(&ga, &gb, x, y, win, off);
                if s < best {
                    best = s;
                    best_off = off;
                }
            }
            let idx = (y as u32 * sw + x as u32) as usize;
            let mag = (best_off * scale as i32).abs().min(255) as u8;
            flow.put_pixel(x as u32, y as u32, image::Luma([mag]));
            sad_min[idx] = best;
            sum_min += best;
            n_visited += 1;
        }
    }

    // Reliability: a block whose best match is far above the image mean is
    // featureless (untextured sky / flat area); displace by reliable-neighbor median.
    let mean_min: f64 = sum_min / n_visited.max(1) as f64;
    let reli: Vec<bool> = sad_min
        .iter()
        .map(|&s| s != f64::MAX && s <= 1.6 * mean_min)
        .collect();
    let filled = reliable_median_fill(&flow, &reli, sw as usize, sh as usize);
    Ok(smear_borders(&filled, win_half as u32))
}

fn sad_window(a: &GrayImage, b: &GrayImage, cx: i32, cy: i32, win: u32, off: i32) -> f64 {
    let half = (win / 2) as i32;
    let mut s = 0f64;
    for dy in -half..=half {
        for dx in -half..=half {
            let py = (cy + dy).clamp(0, a.height() as i32 - 1) as u32;
            let px = (cx + dx).clamp(0, a.width() as i32 - 1) as u32;
            let qx = (cx + dx + off).clamp(0, b.width() as i32 - 1) as u32;
            let qy = (cy + dy).clamp(0, b.height() as i32 - 1) as u32;
            let va = a.get_pixel(px, py)[0] as i32;
            let vb = b.get_pixel(qx, qy)[0] as i32;
            s += (va - vb).abs() as f64;
        }
    }
    s
}

/// Replace unreliable displacement pixels (featureless sky etc.) with the median of
/// reliable 3x3 neighbors; passes over the whole image once.
fn reliable_median_fill(flow: &GrayImage, reli: &[bool], w: usize, h: usize) -> GrayImage {
    let mut out = flow.clone();
    for y in 1..h.saturating_sub(1) {
        for x in 1..w.saturating_sub(1) {
            let idx = y * w + x;
            if reli[idx] {
                continue;
            }
            let mut vals: Vec<u8> = Vec::with_capacity(9);
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = (x as i32 + dx) as usize;
                    let ny = (y as i32 + dy) as usize;
                    if reli[ny * w + nx] {
                        vals.push(flow.get_pixel(nx as u32, ny as u32)[0]);
                    }
                }
            }
            if let Some(v) = median8(&vals) {
                out.put_pixel(x as u32, y as u32, image::Luma([v]));
            }
        }
    }
    out
}

fn median8(vals: &[u8]) -> Option<u8> {
    if vals.is_empty() {
        return None;
    }
    let mut v = vals.to_vec();
    v.sort_unstable();
    Some(v[v.len() / 2])
}

/// Fill the unvisited search border (width `pad` px) from the nearest interior
/// value; otherwise the 0-value border ring becomes a spurious k-means cluster.
fn smear_borders(img: &GrayImage, pad: u32) -> GrayImage {
    let w = img.width();
    let h = img.height();
    let mut out = img.clone();
    let pad = pad.max(1).min(w.min(h).saturating_sub(1));
    let x_lo = pad;
    let x_hi = w.saturating_sub(1).saturating_sub(pad);
    let y_lo = pad;
    let y_hi = h.saturating_sub(1).saturating_sub(pad);
    for y in 0..h {
        for x in 0..w {
            if x >= x_lo && x <= x_hi && y >= y_lo && y <= y_hi {
                continue;
            }
            let nx = x.clamp(x_lo, x_hi);
            let ny = y.clamp(y_lo, y_hi);
            out.put_pixel(x, y, *img.get_pixel(nx, ny));
        }
    }
    out
}

/// 1-D k-means over displacement magnitudes. Labels 0..bands-1 ascending centre
/// order (0 = far, bands-1 = near). Rejects empty bands and single-plane inputs.
///
/// NOTE: plain k-means traps on bad init (e.g. 3 narrow modes {4,10,20} collapsed
/// to {6.8,19.2,24.6} with evenly-spaced seeds). We run 3 deterministic inits and
/// keep the lowest-SSE one; the single-plane verdict uses the robust p5..p95 width
/// so a few aliased edge pixels (window clamping) cannot mask a flat displacement.
fn cluster_displacements(img: &GrayImage, bands: usize) -> Result<(Vec<u8>, Vec<f32>), String> {
    let vals: Vec<u8> = img.as_raw().iter().copied().collect();
    if vals.is_empty() {
        return Err("empty displacement image".to_string());
    }
    let n = vals.len() as f32;
    let mean: f32 = vals.iter().map(|&v| v as f32).sum::<f32>() / n;
    let min = *vals.iter().min().unwrap() as f32;
    let max = *vals.iter().max().unwrap() as f32;

    // Robust p5/p95 width: single-plane reject ignores edge aliasing outliers.
    let mut sorted = vals.clone();
    sorted.sort_unstable();
    let q = |q: f32| -> f32 {
        let i = (q * (sorted.len() - 1) as f32) as usize;
        sorted[i] as f32
    };
    if q(0.05) == q(0.5) && q(0.95) - q(0.05) < 2.0 {
        return Err(format!(
            "displacement range too narrow (median {:.0}px): single depth plane, cannot recover {} layers",
            q(0.5), bands
        ));
    }

    // Initializations (deterministic; kmeans++ is randomized -> non-reproducible runs).
    let inits: Vec<Vec<f32>> = vec![
        // A: even spacing across [min, max].
        (0..bands)
            .map(|b| min + (max - min) * (b as f32 + 0.5) / bands as f32)
            .collect(),
        // B: even spacing across [p5, p95] (robust spread).
        {
            let lo = q(0.05);
            let hi = q(0.95);
            (0..bands)
                .map(|b| lo + (hi - lo) * (b as f32 + 0.5) / bands as f32)
                .collect()
        },
        // C: mean-centered fan (covers a heavy-mode + thin-tail shape).
        (0..bands)
            .map(|b| {
                let off = (b as f32 - (bands as f32 - 1.0) / 2.0) / bands as f32;
                (mean + off * (max - min)).clamp(min, max)
            })
            .collect(),
    ];

    let (mut centers, mut sse_best) = (inits[0].clone(), f32::INFINITY);
    for init in &inits {
        let mut c = init.clone();
        let mut labels = vec![0u8; vals.len()];
        for _ in 0..120 {
            let mut sums = vec![0f32; bands];
            let mut counts = vec![0u32; bands];
            for (i, &v) in vals.iter().enumerate() {
                let b = nearest_center(v as f32, &c);
                labels[i] = b as u8;
                sums[b] += v as f32;
                counts[b] += 1;
            }
            let mut moved = false;
            for b in 0..bands {
                if counts[b] > 0 {
                    let nc = sums[b] / counts[b] as f32;
                    if (nc - c[b]).abs() > 0.005 {
                        c[b] = nc;
                        moved = true;
                    }
                }
            }
            if !moved {
                break;
            }
        }
        let sse: f32 = vals
            .iter()
            .enumerate()
            .map(|(i, &v)| (v as f32 - c[labels[i] as usize]).powi(2))
            .sum();
        if sse < sse_best {
            sse_best = sse;
            centers = c;
        }
    }

    // Re-label with the winning centers; order ascending (0 = far).
    let mut order: Vec<usize> = (0..bands).collect();
    order.sort_by(|&x, &y| centers[x].partial_cmp(&centers[y]).unwrap());
    centers = order.iter().map(|&o| centers[o]).collect();

    let mut labels = vec![0u8; vals.len()];
    let mut counts = vec![0u32; bands];
    for (i, v) in vals.iter().enumerate() {
        let b = nearest_center(*v as f32, &centers);
        labels[i] = b as u8;
        counts[b] += 1;
    }
    for (b, &c) in counts.iter().enumerate() {
        if c == 0 {
            return Err(format!(
                "band {} empty (mean displacement ~{:.1}px); try --bands 2 or a wider/different pan pair",
                b, mean
            ));
        }
    }

    Ok((labels, centers))
}

fn nearest_center(v: f32, c: &[f32]) -> usize {
    let mut best = 0usize;
    let mut bd = f32::INFINITY;
    for (b, &cc) in c.iter().enumerate() {
        let d = (v - cc).abs();
        if d < bd {
            bd = d;
            best = b;
        }
    }
    best
}

fn nearest_upsample(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dw * dh) as usize];
    for y in 0..dh {
        let sy = (y * sh / dh).min(sh - 1);
        for x in 0..dw {
            let sx = (x * sw / dw).min(sw - 1);
            out[(y * dw + x) as usize] = src[(sy * sw + sx) as usize];
        }
    }
    out
}

fn compose_layers(
    src: &DynamicImage,
    labels: &[u8],
    bands: usize,
) -> Result<Vec<RgbaImage>, String> {
    let w = src.width();
    let h = src.height();
    if labels.len() != (w * h) as usize {
        return Err(format!(
            "label count {} != pixels {}x{}",
            labels.len(),
            w,
            h
        ));
    }
    let rgba = src.to_rgba8();
    let mut masks = vec![vec![0f32; labels.len()]; bands];
    for (i, &l) in labels.iter().enumerate() {
        masks[l as usize][i] = 1.0f32;
    }
    let mut out = Vec::with_capacity(bands);
    for mask in masks {
        out.push(feathered_layer(&rgba, &mask, w, h));
    }
    Ok(out)
}

/// Per-band RGBA image where membership alpha is softened by one 3x3 box pass
/// (~2-4px feathered seams between layers).
fn feathered_layer(rgba: &RgbaImage, mask: &[f32], w: u32, h: u32) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0f32;
            let mut n = 0f32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                        continue;
                    }
                    acc += mask[(ny as u32 * w + nx as u32) as usize];
                    n += 1.0;
                }
            }
            let a = (acc / n * 255.0).round() as u8;
            let px = rgba.get_pixel(x, y);
            img.put_pixel(x, y, image::Rgba([px[0], px[1], px[2], a]));
        }
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic, non-periodic pattern so block matching has exactly ONE good
    /// offset per region (periodic test content aliases, see preplan comment).
    fn pat(x: u32, y: u32) -> u8 {
        ((x * 197u32 + y * 217u32 + (x ^ y) * 31u32) & 0xFF) as u8
    }

    /// Frame pair where each x-region is shifted by `dx` between the two frames.
    fn frames_with_plan(plan: &[(u32, u32, i32)]) -> (DynamicImage, DynamicImage) {
        let w = 128u32;
        let h = 64u32;
        let mut a = RgbaImage::new(w, h);
        let mut b = RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let &(_, _, dx) = plan.iter().find(|&&(x0, x1, _)| x >= x0 && x < x1).unwrap();
                let v_a = pat(x.wrapping_add(dx as u32), y);
                let v_b = pat(x, y);
                a.put_pixel(x, y, image::Rgba([v_a, v_a, v_a, 255]));
                b.put_pixel(x, y, image::Rgba([v_b, v_b, v_b, 255]));
            }
        }
        (DynamicImage::ImageRgba8(a), DynamicImage::ImageRgba8(b))
    }

    #[test]
    fn flow_recovers_two_planes() {
        let (a, b) = frames_with_plan(&[(0, 64, 4), (64, 128, 16)]);
        let flow = horizontal_flow(&a, &b, 1).expect("flow");
        let raw = flow.as_raw();
        let (w, h) = (flow.width(), flow.height());
        let mut left = Vec::new();
        let mut right = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let v = raw[(y * w + x) as usize];
                if x < w / 2 {
                    left.push(v);
                } else {
                    right.push(v);
                }
            }
        }
        assert!(!left.is_empty() && !right.is_empty());
        let left_mid = median8(&left).unwrap();
        let right_mid = median8(&right).unwrap();
        assert!((left_mid as i32 - 4).abs() <= 2, "left ~4 got {}", left_mid);
        assert!(
            (right_mid as i32 - 16).abs() <= 2,
            "right ~16 got {}",
            right_mid
        );
    }

    #[test]
    fn clusters_and_layers_have_holes() {
        // Three unambiguous planes: far pan 4px, mid 10px, near 20px.
        let (a, b) = frames_with_plan(&[(0, 43, 4), (43, 85, 10), (85, 128, 20)]);
        let flow = horizontal_flow(&a, &b, 1).expect("flow");
        let (labels, centers) = cluster_displacements(&flow, 3).expect("clusters");
        assert!(centers[0] <= centers[1] && centers[1] <= centers[2]);
        for w2 in centers.windows(2) {
            assert!(w2[1] - w2[0] > 1.0, "centers too close: {:?}", centers);
        }
        let w = flow.width();
        let h = flow.height();
        let mut hist = [[0u32; 3]; 3]; // region x plane-label
        for y in 0..h {
            for x in 0..w {
                let l = labels[(y * w + x) as usize];
                let region = if x < 43 {
                    0
                } else if x < 85 {
                    1
                } else {
                    2
                };
                hist[region][l as usize] += 1;
            }
        }
        for r in 0..3 {
            assert!(
                hist[r][r] > (w * h / 3) as u32 / 2,
                "region {} not dominant at its band: {:?}",
                r,
                hist[r]
            );
        }
        let layers = compose_layers(&a, &labels, 3).expect("layers");
        let mid_y = h / 2;
        let far_a_far = layers[0].get_pixel(8, mid_y)[3];
        let near_a_far = layers[2].get_pixel(8, mid_y)[3];
        let far_a_near = layers[0].get_pixel(100, mid_y)[3];
        let near_a_near = layers[2].get_pixel(100, mid_y)[3];
        assert!(
            far_a_far > 200,
            "far layer alpha {} at far pixel",
            far_a_far
        );
        assert!(
            near_a_far < 60,
            "near layer alpha {} at far pixel",
            near_a_far
        );
        assert!(
            near_a_near > 200,
            "near layer alpha {} at near pixel",
            near_a_near
        );
        assert!(
            far_a_near < 60,
            "far layer alpha {} at near pixel",
            far_a_near
        );
    }

    #[test]
    fn single_plane_is_rejected() {
        let (a, b) = frames_with_plan(&[(0, 128, 10)]);
        let flow = horizontal_flow(&a, &b, 1).expect("flow");
        assert!(
            cluster_displacements(&flow, 3).is_err(),
            "3 bands on 1 plane must fail"
        );
    }

    #[test]
    fn upsample_matches_dimensions() {
        let src = vec![0u8, 1, 2, 3];
        let out = nearest_upsample(&src, 2, 2, 4, 4);
        assert_eq!(out.len(), 16);
        assert_eq!(out[0], 0);
        assert_eq!(out[15], 3);
    }
}
