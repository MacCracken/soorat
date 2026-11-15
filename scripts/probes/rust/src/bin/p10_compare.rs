//! P10 (comparison): chitra 1.4.1's RGBA8 (`out/p10/chitra/`) against `image` 0.25.10's
//! (`out/p10/image/`) and against the ground truth (`fixtures/p10/<f>.expect.rgba`).
//!
//! Lossless rows: bit-identity of chitra vs image, and of each vs the expected pixels; the
//! first differing pixel is named. JPEG rows: per-channel mean / max absolute error against
//! the source for both decoders, the % of pixels with any RGB channel off, chitra vs image
//! directly, and selah's B-img bar (chitra mean <= image mean + 0.05, max <= image max + 2;
//! conformance |chitra - image| <= 2 where neither upsamples chroma).
use std::path::PathBuf;

fn read(p: &PathBuf) -> Vec<u8> {
    std::fs::read(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// First differing pixel: (x, y, channel, a, b).
type Diff = (usize, usize, usize, u8, u8);

/// (count of differing bytes, first differing pixel)
fn first_diff(a: &[u8], b: &[u8], w: usize) -> (usize, Option<Diff>) {
    let mut n = 0;
    let mut first = None;
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        if x != y {
            n += 1;
            if first.is_none() {
                let p = i / 4;
                first = Some((p % w, p / w, i % 4, *x, *y));
            }
        }
    }
    (n, first)
}

struct Err3 {
    mean: [f64; 3],
    max: [u8; 3],
    rgb_mean: f64,
    rgb_max: u8,
    pct_px: f64,
}

fn err_rgb(a: &[u8], b: &[u8]) -> Err3 {
    let mut sum = [0u64; 3];
    let mut max = [0u8; 3];
    let mut px = 0usize;
    let n = a.len() / 4;
    for i in 0..n {
        let mut off = false;
        for c in 0..3 {
            let d = a[i * 4 + c].abs_diff(b[i * 4 + c]);
            sum[c] += u64::from(d);
            max[c] = max[c].max(d);
            off |= d != 0;
        }
        px += usize::from(off);
    }
    let mean = sum.map(|s| s as f64 / n as f64);
    Err3 {
        mean,
        max,
        rgb_mean: (mean[0] + mean[1] + mean[2]) / 3.0,
        rgb_max: *max.iter().max().unwrap_or(&0),
        pct_px: 100.0 * px as f64 / n as f64,
    }
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets");
    let fx = root.join("fixtures/p10");
    let list = std::fs::read_to_string(fx.join("LIST")).expect("LIST");
    let (mut lossless_same, mut lossless_n, mut jpeg_pass, mut jpeg_n) = (0, 0, 0, 0);
    println!("== lossless (PNG, BMP): chitra vs image, and each vs the expected pixels");
    for line in list.lines() {
        let f: Vec<&str> = line.split(' ').collect();
        let (name, w, kind) = (f[0], f[1].parse::<usize>().expect("w"), f[3]);
        let img = read(&root.join(format!("out/p10/image/{name}.rgba")));
        let chi = read(&root.join(format!("out/p10/chitra/{name}.rgba")));
        let exp = read(&fx.join(format!("{name}.expect.rgba")));
        if kind != "lossless" {
            continue;
        }
        lossless_n += 1;
        let (d_ci, f_ci) = first_diff(&chi, &img, w);
        let (d_ce, f_ce) = first_diff(&chi, &exp, w);
        let (d_ie, f_ie) = first_diff(&img, &exp, w);
        if d_ci == 0 {
            lossless_same += 1;
        }
        let show = |d: usize, f: Option<Diff>| match f {
            None => "identical".to_string(),
            Some((x, y, c, a, b)) => {
                format!("{d} bytes differ, first at ({x},{y}) ch{c}: {a} vs {b}")
            }
        };
        println!(
            "{name:32} chitra~image: {:44} chitra~expect: {:40} image~expect: {}",
            show(d_ci, f_ci),
            show(d_ce, f_ce),
            show(d_ie, f_ie)
        );
    }
    println!("lossless bit-identical chitra == image: {lossless_same} of {lossless_n}");

    println!(
        "\n== JPEG: error against the source pixels (R G B mean | max), RGB mean/max, % pixels off"
    );
    for line in list.lines() {
        let f: Vec<&str> = line.split(' ').collect();
        let (name, kind) = (f[0], f[3]);
        if kind != "jpeg" {
            continue;
        }
        jpeg_n += 1;
        let img = read(&root.join(format!("out/p10/image/{name}.rgba")));
        let chi = read(&root.join(format!("out/p10/chitra/{name}.rgba")));
        let src = read(&fx.join(format!("{name}.expect.rgba")));
        let ei = err_rgb(&img, &src);
        let ec = err_rgb(&chi, &src);
        let cc = err_rgb(&chi, &img);
        let alpha_ok = chi.chunks(4).all(|p| p[3] == 255) && img.chunks(4).all(|p| p[3] == 255);
        let acc =
            ec.rgb_mean <= ei.rgb_mean + 0.05 && u16::from(ec.rgb_max) <= u16::from(ei.rgb_max) + 2;
        let conf = cc.rgb_max <= 2;
        jpeg_pass += usize::from(acc && conf);
        let fmt = |e: &Err3| {
            format!(
                "{:.3} {:.3} {:.3} | {:3} {:3} {:3} | {:.3}/{:3} | {:5.1}%",
                e.mean[0],
                e.mean[1],
                e.mean[2],
                e.max[0],
                e.max[1],
                e.max[2],
                e.rgb_mean,
                e.rgb_max,
                e.pct_px
            )
        };
        println!("{name}");
        println!("    image  vs source: {}", fmt(&ei));
        println!("    chitra vs source: {}", fmt(&ec));
        println!(
            "    chitra vs image : max |d| {} (R{} G{} B{}), {:.2}% pixels differ, mean {:.4}; alpha 255 both: {alpha_ok}",
            cc.rgb_max, cc.max[0], cc.max[1], cc.max[2], cc.pct_px, cc.rgb_mean
        );
        println!(
            "    B-img accuracy: mean {:+.4} (bar +0.05), max {:+} (bar +2) -> {}; conformance K_jpeg=2 -> {}",
            ec.rgb_mean - ei.rgb_mean,
            i16::from(ec.rgb_max) - i16::from(ei.rgb_max),
            if acc { "pass" } else { "FAIL" },
            if conf { "pass" } else { "FAIL" }
        );
    }
    println!("JPEG rows passing B-img accuracy and conformance: {jpeg_pass} of {jpeg_n}");

    println!(
        "\n== JPEG with chroma subsampling (cjpeg q90): each fixture and its 180-degree partner; RGB mean/max vs source"
    );
    let (mut sub_pass, mut sub_n) = (0, 0);
    for line in list.lines() {
        let f: Vec<&str> = line.split(' ').collect();
        let (name, kind) = (f[0], f[3]);
        if kind != "jpegsub" || name.contains(".rot180.") {
            continue;
        }
        sub_n += 1;
        let dot = name.find('.').unwrap_or(0);
        let partner = format!("{}.rot180{}", &name[..dot], &name[dot..]);
        let load = |n: &str| {
            (
                read(&root.join(format!("out/p10/image/{n}.rgba"))),
                read(&root.join(format!("out/p10/chitra/{n}.rgba"))),
                read(&fx.join(format!("{n}.expect.rgba"))),
            )
        };
        let (i1, c1, s1) = load(name);
        let (i2, c2, s2) = load(&partner);
        let (ei1, ec1, cc1) = (err_rgb(&i1, &s1), err_rgb(&c1, &s1), err_rgb(&c1, &i1));
        let (ei2, ec2, cc2) = (err_rgb(&i2, &s2), err_rgb(&c2, &s2), err_rgb(&c2, &i2));
        let pm_c = (ec1.rgb_mean + ec2.rgb_mean) / 2.0;
        let pm_i = (ei1.rgb_mean + ei2.rgb_mean) / 2.0;
        let mx_c = ec1.rgb_max.max(ec2.rgb_max);
        let mx_i = ei1.rgb_max.max(ei2.rgb_max);
        let pass = pm_c <= pm_i + 0.05 && u16::from(mx_c) <= u16::from(mx_i) + 2;
        sub_pass += usize::from(pass);
        println!(
            "{name:34} chitra {:.3}/{:3} turned {:.3}/{:3} | image {:.3}/{:3} turned {:.3}/{:3} | pair mean chitra {pm_c:.3} image {pm_i:.3} ({:+.3}), worst max {mx_c} vs {mx_i} -> {} | chitra~image max |d| {}/{}",
            ec1.rgb_mean,
            ec1.rgb_max,
            ec2.rgb_mean,
            ec2.rgb_max,
            ei1.rgb_mean,
            ei1.rgb_max,
            ei2.rgb_mean,
            ei2.rgb_max,
            pm_c - pm_i,
            if pass { "pass" } else { "FAIL" },
            cc1.rgb_max,
            cc2.rgb_max
        );
    }
    println!("subsampled JPEG pairs passing B-img (pair rule): {sub_pass} of {sub_n}");
}
