//! soorat class-G render-reference corpus (M0 step 6 of the Cyrius port).
//!
//! Renders the scenes in `tests/golden/render/scenes.json` offscreen with the
//! frozen Rust soorat 1.2.0 oracle and records each RGBA8 readback.
//!
//! ```text
//! render_ref [--corpus DIR]          render twice, prove the runs identical, write the corpus
//! render_ref --check [--corpus DIR]  re-render and compare with the stored corpus
//! ```
//!
//! Exit codes: 0 identical / written; 1 a difference (or a non-deterministic
//! run, or a stale corpus); 2 refused (another adapter, driver or stack, no
//! adapter, no corpus, bad arguments or a bad scenes.json).

mod render;
mod scenes;

use render::{Rendered, Uploads};
use scenes::SceneFile;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const MANIFEST: &str = "manifest.tsv";
const UPLOADS: &str = "uploads.tsv";
const SCENES: &str = "scenes.json";
/// Uploads up to this size are also written out as hex.
const HEX_LIMIT: usize = 256;

const ORACLE_MANIFEST: &str = include_str!("../../../rust-old/Cargo.toml");
const LOCK: &str = include_str!("../Cargo.lock");

fn main() -> ExitCode {
    let mut check = false;
    let mut corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/render");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--check" => check = true,
            "--corpus" => match args.next() {
                Some(d) => corpus = PathBuf::from(d),
                None => return refuse("--corpus needs a directory"),
            },
            "-h" | "--help" => {
                println!("usage: render_ref [--check] [--corpus DIR]");
                return ExitCode::SUCCESS;
            }
            other => return refuse(&format!("unknown argument {other:?}")),
        }
    }
    let corpus = corpus.canonicalize().unwrap_or(corpus);

    let scenes_text = match std::fs::read_to_string(corpus.join(SCENES)) {
        Ok(t) => t,
        Err(e) => return refuse(&format!("{}: {e}", corpus.join(SCENES).display())),
    };
    let file = match scenes::load(&scenes_text) {
        Ok(f) => f,
        Err(e) => return refuse(&e),
    };
    let scenes_sha = sha256_hex(scenes_text.as_bytes());

    let ctx = match gpu() {
        Ok(c) => c,
        Err(e) => return refuse(&e),
    };
    let platform = platform(&ctx.adapter_info());
    println!("{:<12}{}", "corpus", corpus.display());
    for (k, v) in &platform {
        println!("{k:<12}{v}");
    }

    if check {
        check_corpus(&ctx, &file, &corpus, &platform, &scenes_sha)
    } else {
        write_corpus(ctx, &file, &corpus, &platform, &scenes_sha)
    }
}

fn refuse(msg: &str) -> ExitCode {
    eprintln!("render_ref: {msg}");
    ExitCode::from(2)
}

fn gpu() -> Result<soorat::GpuContext, String> {
    pollster::block_on(soorat::GpuContext::new()).map_err(|e| format!("no GPU adapter: {e}"))
}

// ── Provenance ──────────────────────────────────────────────────────────────

/// The adapter, driver and stack. A corpus only replays where all of these match.
fn platform(info: &wgpu::AdapterInfo) -> Vec<(&'static str, String)> {
    let mesa = info
        .driver_info
        .split_whitespace()
        .skip_while(|w| *w != "Mesa")
        .nth(1)
        .unwrap_or("-")
        .to_string();
    vec![
        ("adapter", info.name.clone()),
        ("vendor", format!("0x{:04x}", info.vendor)),
        ("device", format!("0x{:04x}", info.device)),
        ("type", format!("{:?}", info.device_type)),
        ("backend", format!("{:?}", info.backend)),
        ("driver", info.driver.clone()),
        ("driver_info", info.driver_info.clone()),
        ("mesa", mesa),
        ("glibc", glibc()),
        ("wgpu", locked_version("wgpu")),
        ("soorat", oracle_version()),
    ]
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn glibc() -> String {
    unsafe extern "C" {
        fn gnu_get_libc_version() -> *const std::ffi::c_char;
    }
    // SAFETY: glibc returns a pointer to a static NUL-terminated string.
    unsafe { std::ffi::CStr::from_ptr(gnu_get_libc_version()) }
        .to_string_lossy()
        .into_owned()
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
fn glibc() -> String {
    "-".into()
}

/// A package's version in this crate's Cargo.lock (the oracle's, seeded).
fn locked_version(name: &str) -> String {
    let needle = format!("name = \"{name}\"\nversion = \"");
    LOCK.find(&needle)
        .and_then(|i| LOCK[i + needle.len()..].split('"').next())
        .unwrap_or("-")
        .to_string()
}

fn oracle_version() -> String {
    ORACLE_MANIFEST
        .lines()
        .find_map(|l| l.strip_prefix("version = \""))
        .and_then(|v| v.split('"').next())
        .unwrap_or("-")
        .to_string()
}

// ── Rendering ───────────────────────────────────────────────────────────────

fn render_all(ctx: &soorat::GpuContext, file: &SceneFile) -> Result<Vec<Rendered>, String> {
    file.scenes
        .iter()
        .map(|s| render::render_scene(ctx, file, s).map_err(|e| format!("{}: {e}", s.name)))
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Pixel-level comparison: (differing pixels, max channel delta, first differing pixel).
fn diff(a: &[u8], b: &[u8], width: u32) -> (usize, u8, Option<(u32, u32)>) {
    let (mut n, mut max, mut first) = (0, 0u8, None);
    for (i, (pa, pb)) in a
        .as_chunks::<4>()
        .0
        .iter()
        .zip(b.as_chunks::<4>().0)
        .enumerate()
    {
        if pa != pb {
            n += 1;
            let d = pa
                .iter()
                .zip(pb)
                .map(|(x, y)| x.abs_diff(*y))
                .max()
                .unwrap_or(0);
            max = max.max(d);
            if first.is_none() {
                let i = i as u32;
                first = Some((i % width, i / width));
            }
        }
    }
    (n, max, first)
}

fn uploads_diff(a: &Uploads, b: &Uploads) -> Vec<String> {
    let mut out = Vec::new();
    if a.0.len() != b.0.len() {
        out.push(format!("{} uploads vs {}", a.0.len(), b.0.len()));
    }
    for ((na, ba), (nb, bb)) in a.0.iter().zip(&b.0) {
        if na != nb || ba != bb {
            out.push(format!("upload {na} differs"));
        }
    }
    out
}

fn distinct_colors(rgba: &[u8]) -> usize {
    let mut v: Vec<[u8; 4]> = rgba.as_chunks::<4>().0.to_vec();
    v.sort_unstable();
    v.dedup();
    v.len()
}

// ── Write ───────────────────────────────────────────────────────────────────

fn write_corpus(
    ctx: soorat::GpuContext,
    file: &SceneFile,
    corpus: &Path,
    platform: &[(&'static str, String)],
    scenes_sha: &str,
) -> ExitCode {
    // Run 1 and run 2, each on its own device; both must be byte-identical.
    let first = match render_all(&ctx, file) {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    drop(ctx);
    let ctx2 = match gpu() {
        Ok(c) => c,
        Err(e) => return refuse(&e),
    };
    if platform != platform_of(&ctx2) {
        return refuse("the second device reports a different adapter");
    }
    let second = match render_all(&ctx2, file) {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    let mut nondeterministic = 0;
    for ((scene, a), b) in file.scenes.iter().zip(&first).zip(&second) {
        let (n, max, at) = diff(&a.rgba, &b.rgba, scene.width);
        let ups = uploads_diff(&a.uploads, &b.uploads);
        if n > 0 || !ups.is_empty() {
            nondeterministic += 1;
            println!(
                "NONDETERMINISTIC {}: {n} pixels differ (max Δ {max}, first at {at:?}); {}",
                scene.name,
                ups.join("; ")
            );
        }
    }
    if nondeterministic > 0 {
        return fail(&format!(
            "{nondeterministic} scene(s) differ between two runs; nothing written"
        ));
    }
    println!(
        "determinism: run 1 and run 2 (separate devices) are byte-identical for all {} scenes",
        file.scenes.len()
    );

    if let Err(e) = std::fs::create_dir_all(corpus) {
        return fail(&format!("{}: {e}", corpus.display()));
    }
    // PNGs of scenes no longer in scenes.json would go stale: remove them.
    if let Ok(entries) = std::fs::read_dir(corpus) {
        for e in entries.flatten() {
            let p = e.path();
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if p.extension().is_some_and(|x| x == "png")
                && !file.scenes.iter().any(|s| s.name == stem)
            {
                let _ = std::fs::remove_file(&p);
            }
        }
    }

    let mut manifest = String::new();
    let _ = writeln!(
        manifest,
        "# soorat class-G render-reference corpus — generated by scripts/render_ref; do not edit"
    );
    let _ = writeln!(
        manifest,
        "# Replays only on the adapter, driver and stack below (render_ref --check refuses otherwise)."
    );
    for (k, v) in platform {
        let _ = writeln!(manifest, "# {k}\t{v}");
    }
    let _ = writeln!(manifest, "# scenes_json_sha256\t{scenes_sha}");
    for s in &file.skipped {
        let _ = writeln!(manifest, "# skipped\t{}\t{}", s.name, s.reason);
    }
    let _ = writeln!(
        manifest,
        "scene\twidth\theight\tformat\tsha256\tdescription"
    );

    let mut uploads = String::new();
    let _ = writeln!(
        uploads,
        "# CPU-side bytes each scene uploads, in upload order — generated by scripts/render_ref; do not edit"
    );
    let _ = writeln!(
        uploads,
        "# hex is the little-endian buffer content when it is at most {HEX_LIMIT} bytes, else -"
    );
    let _ = writeln!(uploads, "scene\tupload\tbytes\tsha256\thex");

    let mut total = 0u64;
    for (scene, r) in file.scenes.iter().zip(&first) {
        let png = match encode_png(&r.rgba, scene.width, scene.height) {
            Ok(p) => p,
            Err(e) => return fail(&format!("{}: png: {e}", scene.name)),
        };
        let path = corpus.join(format!("{}.png", scene.name));
        if let Err(e) = std::fs::write(&path, &png) {
            return fail(&format!("{}: {e}", path.display()));
        }
        total += png.len() as u64;
        let sha = sha256_hex(&r.rgba);
        let _ = writeln!(
            manifest,
            "{}\t{}\t{}\t{}\t{}\t{}",
            scene.name,
            scene.width,
            scene.height,
            scene.format.name(),
            sha,
            scene.description
        );
        for (name, bytes) in &r.uploads.0 {
            let h = if bytes.len() <= HEX_LIMIT {
                hex(bytes)
            } else {
                "-".into()
            };
            let _ = writeln!(
                uploads,
                "{}\t{name}\t{}\t{}\t{h}",
                scene.name,
                bytes.len(),
                sha256_hex(bytes)
            );
        }
        println!(
            "wrote {:<22} {:>6} B png  {} colors  sha256 {}",
            scene.name,
            png.len(),
            distinct_colors(&r.rgba),
            &sha[..16]
        );
    }
    for s in &file.skipped {
        println!("skipped {:<20} {}", s.name, s.reason);
    }
    for (name, text) in [(MANIFEST, &manifest), (UPLOADS, &uploads)] {
        let path = corpus.join(name);
        if let Err(e) = std::fs::write(&path, text) {
            return fail(&format!("{}: {e}", path.display()));
        }
        total += text.len() as u64;
    }
    total += std::fs::metadata(corpus.join(SCENES))
        .map(|m| m.len())
        .unwrap_or(0);
    println!(
        "corpus: {} scenes, {total} bytes (PNGs + manifest + uploads + scenes.json)",
        file.scenes.len()
    );
    ExitCode::SUCCESS
}

fn platform_of(ctx: &soorat::GpuContext) -> Vec<(&'static str, String)> {
    platform(&ctx.adapter_info())
}

fn fail(msg: &str) -> ExitCode {
    eprintln!("render_ref: {msg}");
    ExitCode::from(1)
}

fn encode_png(rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>, String> {
    use image::ImageEncoder;
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let mut out = Vec::new();
    PngEncoder::new_with_quality(&mut out, CompressionType::Best, FilterType::Adaptive)
        .write_image(rgba, w, h, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

fn decode_png(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let (w, h) = (img.width(), img.height());
    Ok((w, h, img.into_rgba8().into_raw()))
}

// ── Check ───────────────────────────────────────────────────────────────────

struct Manifest {
    header: Vec<(String, String)>,
    rows: Vec<Vec<String>>,
}

fn read_tsv(path: &Path) -> Result<Manifest, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut header = Vec::new();
    let mut rows = Vec::new();
    let mut seen_columns = false;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            if let Some((k, v)) = rest.split_once('\t') {
                header.push((k.to_string(), v.to_string()));
            }
        } else if !line.is_empty() {
            if seen_columns {
                rows.push(line.split('\t').map(str::to_string).collect());
            } else {
                seen_columns = true;
            }
        }
    }
    Ok(Manifest { header, rows })
}

fn check_corpus(
    ctx: &soorat::GpuContext,
    file: &SceneFile,
    corpus: &Path,
    platform: &[(&'static str, String)],
    scenes_sha: &str,
) -> ExitCode {
    let manifest = match read_tsv(&corpus.join(MANIFEST)) {
        Ok(m) => m,
        Err(e) => {
            return refuse(&format!(
                "no corpus to check ({e}); run without --check first"
            ));
        }
    };
    let recorded = |k: &str| {
        manifest
            .header
            .iter()
            .find(|(hk, _)| hk == k)
            .map(|(_, v)| v.as_str())
    };
    let mismatches: Vec<String> = platform
        .iter()
        .filter(|(k, v)| recorded(k) != Some(v.as_str()))
        .map(|(k, v)| {
            format!(
                "{k}: recorded {:?}, this machine {v:?}",
                recorded(k).unwrap_or("(missing)")
            )
        })
        .collect();
    if !mismatches.is_empty() {
        return refuse(&format!(
            "the corpus was recorded on another adapter, driver or stack; class G belongs to one machine:\n  {}",
            mismatches.join("\n  ")
        ));
    }
    if recorded("scenes_json_sha256") != Some(scenes_sha) {
        return fail(
            "scenes.json changed since the corpus was recorded; regenerate it (render_ref without --check)",
        );
    }
    let names: Vec<&str> = manifest.rows.iter().map(|r| r[0].as_str()).collect(); // split() yields ≥ 1 field
    let want: Vec<&str> = file.scenes.iter().map(|s| s.name.as_str()).collect();
    if names != want {
        return fail(&format!(
            "manifest scenes {names:?} != scenes.json {want:?}"
        ));
    }
    let uploads = match read_tsv(&corpus.join(UPLOADS)) {
        Ok(u) => u,
        Err(e) => return fail(&e),
    };

    let mut failures = 0;
    for (scene, row) in file.scenes.iter().zip(&manifest.rows) {
        let r = match render::render_scene(ctx, file, scene) {
            Ok(r) => r,
            Err(e) => return fail(&format!("{}: {e}", scene.name)),
        };
        let mut problems = Vec::new();
        let sha = sha256_hex(&r.rgba);
        if row.get(4).map(String::as_str) != Some(sha.as_str()) {
            problems.push("sha256 differs from the manifest".to_string());
        }
        if row.get(1) != Some(&scene.width.to_string())
            || row.get(2) != Some(&scene.height.to_string())
            || row.get(3).map(String::as_str) != Some(scene.format.name())
        {
            problems.push("size or format differs from the manifest".to_string());
        }
        match decode_png(&corpus.join(format!("{}.png", scene.name))) {
            Ok((w, h, stored)) => {
                if (w, h) != (scene.width, scene.height) || stored.len() != r.rgba.len() {
                    problems.push(format!("stored PNG is {w}×{h}"));
                } else {
                    if row.get(4) != Some(&sha256_hex(&stored)) {
                        problems.push("stored PNG does not match its manifest sha256".into());
                    }
                    let (n, max, at) = diff(&stored, &r.rgba, w);
                    if n > 0 {
                        let total = (w * h) as usize;
                        problems.push(format!(
                            "{n}/{total} pixels differ ({:.3} %), max channel Δ {max}, first at {at:?}",
                            100.0 * n as f64 / total as f64
                        ));
                    }
                }
            }
            Err(e) => problems.push(e),
        }
        let stored_uploads: Vec<&Vec<String>> = uploads
            .rows
            .iter()
            .filter(|u| u.first() == Some(&scene.name))
            .collect();
        if stored_uploads.len() != r.uploads.0.len() {
            problems.push(format!(
                "{} uploads, {} recorded",
                r.uploads.0.len(),
                stored_uploads.len()
            ));
        }
        for (u, (name, bytes)) in stored_uploads.iter().zip(&r.uploads.0) {
            if u.get(1) != Some(name) || u.get(3) != Some(&sha256_hex(bytes)) {
                problems.push(format!("upload {name} differs"));
            }
        }
        if problems.is_empty() {
            println!("ok    {}", scene.name);
        } else {
            failures += 1;
            println!("DIFF  {}: {}", scene.name, problems.join("; "));
        }
    }
    if failures > 0 {
        eprintln!(
            "render_ref: {failures} of {} scenes differ",
            file.scenes.len()
        );
        return ExitCode::from(1);
    }
    println!("all {} scenes identical to the corpus", file.scenes.len());
    ExitCode::SUCCESS
}
