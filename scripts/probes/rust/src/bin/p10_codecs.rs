//! P10 (Rust side): deterministic ground-truth images, encoded as PNG / JPEG / BMP, then
//! decoded the way soorat 1.2.0 decodes textures (`image::load_from_memory(b)?.to_rgba8()`,
//! src/texture.rs).
//!
//! Writes into `../assets/`:
//! - `fixtures/p10/<pattern>.source.rgba` — the pattern's RGBA8 pixels;
//! - `fixtures/p10/<fixture>` — each encoded file;
//! - `fixtures/p10/<fixture>.expect.rgba` — the RGBA8 an exact decoder returns (for JPEG:
//!   the pixels handed to the encoder, alpha 255);
//! - `fixtures/p10/LIST` — `<fixture> <w> <h> <lossless|jpeg>` per line;
//! - `out/p10/image/<fixture>.rgba` — `image` 0.25.10's decode.
//!
//! `image` 0.25.10 cannot write palette, sub-byte or Adam7-interlaced PNGs (its PngEncoder
//! takes L8/La8/Rgb8/Rgba8 and their 16-bit forms, never interlaced). Those four fixtures are
//! assembled here by hand: stored (uncompressed) deflate blocks, CRC-32 and Adler-32 written
//! out, the five PNG filters cycled row by row. No dependency is added.
use std::path::{Path, PathBuf};
use std::time::Instant;

use image::codecs::bmp::BmpEncoder;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

/// RGBA8 pattern pixels.
fn pattern(kind: &str, w: u32, h: u32) -> Vec<u8> {
    let mut rng = Rng(0xD1B5_4A32_D192_ED03 ^ u64::from(w * 131 + h));
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let (r, g, b, a) = match kind {
                "grad" => (
                    (x * 255 / (w - 1)) as u8,
                    (y * 255 / (h - 1)) as u8,
                    (((x + y) * 255) / (w + h - 2)) as u8,
                    (255 - (x * 3 + y * 5) % 200) as u8,
                ),
                "edges" => {
                    let cell = ((x / 5) + (y / 4)) % 2 == 0;
                    let diag = (x as i32 - 2 * y as i32).rem_euclid(13) < 2;
                    if diag {
                        (255, 255, 0, 255)
                    } else if cell {
                        (230, 20, 40, if x % 7 == 0 { 0 } else { 255 })
                    } else {
                        (10, 40, 220, 128)
                    }
                }
                "noise" => {
                    let v = rng.next();
                    (v as u8, (v >> 8) as u8, (v >> 16) as u8, (v >> 24) as u8)
                }
                "photo" => {
                    // smooth, low-noise "photo": two radial blobs + gradient + mild noise
                    let fx = x as f64 / w as f64;
                    let fy = y as f64 / h as f64;
                    let d1 = ((fx - 0.3).powi(2) + (fy - 0.4).powi(2)).sqrt();
                    let d2 = ((fx - 0.7).powi(2) + (fy - 0.6).powi(2)).sqrt();
                    let n = (rng.next() % 9) as f64 - 4.0;
                    let c = |v: f64| v.clamp(0.0, 255.0) as u8;
                    (
                        c(255.0 * (1.0 - d1 * 1.6) + n),
                        c(200.0 * fx + 40.0 * (1.0 - d2 * 2.0) + n),
                        c(255.0 * (1.0 - d2 * 1.4) * fy + n),
                        255,
                    )
                }
                _ => unreachable!(),
            };
            px.extend([r, g, b, a]);
        }
    }
    px
}

fn gray_of(px: &[u8]) -> Vec<u8> {
    px.chunks(4)
        .map(|p| {
            ((u32::from(p[0]) * 299 + u32::from(p[1]) * 587 + u32::from(p[2]) * 114 + 500) / 1000)
                as u8
        })
        .collect()
}

fn rgba_from(f: impl Fn(usize) -> [u8; 4], n: usize) -> Vec<u8> {
    (0..n).flat_map(f).collect()
}

// ------------------------------------------------------------- hand-rolled PNG pieces

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *t = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = table[((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut chunks = data.chunks(65535).peekable();
    if data.is_empty() {
        out.extend([1, 0, 0, 0xFF, 0xFF]);
    }
    while let Some(c) = chunks.next() {
        out.push(u8::from(chunks.peek().is_none()));
        let n = c.len() as u16;
        out.extend(n.to_le_bytes());
        out.extend((!n).to_le_bytes());
        out.extend(c);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + u32::from(x)) % 65521;
        b = (b + a) % 65521;
    }
    out.extend(((b << 16) | a).to_be_bytes());
    out
}

fn chunk(out: &mut Vec<u8>, ty: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    let mut c = ty.to_vec();
    c.extend(data);
    out.extend(&c);
    out.extend(crc32(&c).to_be_bytes());
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i16::from(a) + i16::from(b) - i16::from(c);
    let (pa, pb, pc) = (
        (p - i16::from(a)).abs(),
        (p - i16::from(b)).abs(),
        (p - i16::from(c)).abs(),
    );
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Filter one scanline with type `ft` (0..=4) against `prev` (zeros for the first row).
fn filter_row(ft: u8, row: &[u8], prev: &[u8], bpp: usize, out: &mut Vec<u8>) {
    out.push(ft);
    for i in 0..row.len() {
        let a = if i >= bpp { row[i - bpp] } else { 0 };
        let b = prev[i];
        let c = if i >= bpp { prev[i - bpp] } else { 0 };
        let pred = match ft {
            0 => 0,
            1 => a,
            2 => b,
            3 => ((u16::from(a) + u16::from(b)) / 2) as u8,
            _ => paeth(a, b, c),
        };
        out.push(row[i].wrapping_sub(pred));
    }
}

/// A PNG from packed scanlines (`rows[y]` already at the target bit depth), filters cycled
/// by row; `adam7` re-slices an 8-bit-per-sample image into the seven passes.
#[allow(clippy::too_many_arguments)]
fn png_by_hand(
    w: u32,
    h: u32,
    depth: u8,
    ctype: u8,
    bpp_bytes: usize,
    rows: &[Vec<u8>],
    plte: Option<&[u8]>,
    trns: Option<&[u8]>,
    adam7: bool,
) -> Vec<u8> {
    let mut raw = Vec::new();
    let mut ft = 0u8;
    if adam7 {
        const P: [(u32, u32, u32, u32); 7] = [
            (0, 0, 8, 8),
            (4, 0, 8, 8),
            (0, 4, 4, 8),
            (2, 0, 4, 4),
            (0, 2, 2, 4),
            (1, 0, 2, 2),
            (0, 1, 1, 2),
        ];
        for (x0, y0, dx, dy) in P {
            let pw = if w > x0 { (w - x0).div_ceil(dx) } else { 0 };
            let ph = if h > y0 { (h - y0).div_ceil(dy) } else { 0 };
            if pw == 0 || ph == 0 {
                continue;
            }
            let mut prev = vec![0u8; pw as usize * bpp_bytes];
            for py in 0..ph {
                let y = (y0 + py * dy) as usize;
                let mut row = Vec::with_capacity(pw as usize * bpp_bytes);
                for px in 0..pw {
                    let x = (x0 + px * dx) as usize;
                    row.extend(&rows[y][x * bpp_bytes..(x + 1) * bpp_bytes]);
                }
                filter_row(ft, &row, &prev, bpp_bytes, &mut raw);
                ft = (ft + 1) % 5;
                prev = row;
            }
        }
    } else {
        let mut prev = vec![0u8; rows[0].len()];
        for row in rows {
            filter_row(ft, row, &prev, bpp_bytes, &mut raw);
            ft = (ft + 1) % 5;
            prev.clone_from(row);
        }
    }
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend(w.to_be_bytes());
    ihdr.extend(h.to_be_bytes());
    ihdr.extend([depth, ctype, 0, 0, u8::from(adam7)]);
    chunk(&mut out, b"IHDR", &ihdr);
    if let Some(p) = plte {
        chunk(&mut out, b"PLTE", p);
    }
    if let Some(t) = trns {
        chunk(&mut out, b"tRNS", t);
    }
    chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    chunk(&mut out, b"IEND", &[]);
    out
}

// ------------------------------------------------------------- driver

struct Out {
    dir: PathBuf,
    list: String,
}

impl Out {
    fn put(&mut self, name: &str, bytes: &[u8], w: u32, h: u32, kind: &str, expect: &[u8]) {
        assert_eq!(expect.len(), (w * h * 4) as usize, "{name}: expect size");
        std::fs::write(self.dir.join(name), bytes).expect("write fixture");
        std::fs::write(self.dir.join(format!("{name}.expect.rgba")), expect).expect("write expect");
        self.list.push_str(&format!("{name} {w} {h} {kind}\n"));
    }
}

fn png_std(px: &[u8], w: u32, h: u32, ct: ExtendedColorType) -> Vec<u8> {
    let mut buf = Vec::new();
    PngEncoder::new_with_quality(&mut buf, CompressionType::Default, FilterType::Adaptive)
        .write_image(px, w, h, ct)
        .expect("png encode");
    buf
}

fn rgb_of(px: &[u8]) -> Vec<u8> {
    px.chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect()
}

fn opaque(px: &[u8]) -> Vec<u8> {
    px.chunks(4).flat_map(|p| [p[0], p[1], p[2], 255]).collect()
}

fn gray_rgba(g: &[u8]) -> Vec<u8> {
    g.iter().flat_map(|&v| [v, v, v, 255]).collect()
}

fn emit_pattern(o: &mut Out, kind: &str, w: u32, h: u32, full: bool) {
    let stem = format!("{kind}-{w}x{h}");
    let px = pattern(kind, w, h);
    std::fs::write(o.dir.join(format!("{stem}.source.rgba")), &px).expect("source");
    let n = (w * h) as usize;
    let g = gray_of(&px);
    let rgb = rgb_of(&px);

    // PNG through image's encoder
    o.put(
        &format!("{stem}.rgba8.png"),
        &png_std(&px, w, h, ExtendedColorType::Rgba8),
        w,
        h,
        "lossless",
        &px,
    );
    // JPEG at three qualities (image's encoder: baseline, 4:4:4 — see report) + gray
    for q in [90u8, 75, 50] {
        let mut buf = Vec::new();
        JpegEncoder::new_with_quality(&mut buf, q)
            .encode(&rgb, w, h, ExtendedColorType::Rgb8)
            .expect("jpeg");
        o.put(
            &format!("{stem}.q{q}.jpg"),
            &buf,
            w,
            h,
            "jpeg",
            &opaque(&px),
        );
    }
    if !full {
        return;
    }
    o.put(
        &format!("{stem}.rgb8.png"),
        &png_std(&rgb, w, h, ExtendedColorType::Rgb8),
        w,
        h,
        "lossless",
        &opaque(&px),
    );
    o.put(
        &format!("{stem}.gray8.png"),
        &png_std(&g, w, h, ExtendedColorType::L8),
        w,
        h,
        "lossless",
        &gray_rgba(&g),
    );
    let ga: Vec<u8> = g
        .iter()
        .zip(px.chunks(4))
        .flat_map(|(&v, p)| [v, p[3]])
        .collect();
    let ga_expect: Vec<u8> = g
        .iter()
        .zip(px.chunks(4))
        .flat_map(|(&v, p)| [v, v, v, p[3]])
        .collect();
    o.put(
        &format!("{stem}.graya8.png"),
        &png_std(&ga, w, h, ExtendedColorType::La8),
        w,
        h,
        "lossless",
        &ga_expect,
    );
    // 16-bit RGBA: every sample has a low byte, so the 16 -> 8 narrowing is exercised
    let mut rng = Rng(0x1234_5678_9ABC_DEF1 ^ u64::from(w));
    let s16: Vec<u16> = px
        .iter()
        .map(|&v| u16::from(v) * 256 + (rng.next() & 0xFF) as u16)
        .collect();
    let s16b: Vec<u8> = s16.iter().flat_map(|v| v.to_ne_bytes()).collect();
    let e16: Vec<u8> = s16
        .iter()
        .map(|&v| ((u32::from(v) * 255 + 32767) / 65535) as u8)
        .collect();
    o.put(
        &format!("{stem}.rgba16.png"),
        &png_std(&s16b, w, h, ExtendedColorType::Rgba16),
        w,
        h,
        "lossless",
        &e16,
    );
    // JPEG grayscale
    let mut buf = Vec::new();
    JpegEncoder::new_with_quality(&mut buf, 90)
        .encode(&g, w, h, ExtendedColorType::L8)
        .expect("jpeg gray");
    o.put(
        &format!("{stem}.gray.q90.jpg"),
        &buf,
        w,
        h,
        "jpeg",
        &gray_rgba(&g),
    );
    // BMP: 24-bit, 32-bit (V4 header, alpha mask), 8-bit gray palette
    for (tag, data, ct, exp) in [
        ("rgb24", rgb.clone(), ExtendedColorType::Rgb8, opaque(&px)),
        ("rgba32", px.clone(), ExtendedColorType::Rgba8, px.clone()),
        ("gray8", g.clone(), ExtendedColorType::L8, gray_rgba(&g)),
    ] {
        let mut buf = Vec::new();
        BmpEncoder::new(&mut buf)
            .encode(&data, w, h, ct)
            .expect("bmp");
        o.put(&format!("{stem}.{tag}.bmp"), &buf, w, h, "lossless", &exp);
    }
    // Hand-assembled PNGs (image cannot write these)
    // (a) 8-bit palette, 256 entries, tRNS shorter than the palette (entries >= 200 opaque)
    let pal: Vec<[u8; 3]> = (0..256u32)
        .map(|i| {
            [
                (i * 37 % 256) as u8,
                (i * 91 % 256) as u8,
                (i * 13 % 256) as u8,
            ]
        })
        .collect();
    let trns: Vec<u8> = (0..200u32).map(|i| (255 - i) as u8).collect();
    let idx: Vec<u8> = (0..n)
        .map(|i| {
            (px[i * 4] / 2)
                .wrapping_add(px[i * 4 + 1] / 3)
                .wrapping_add((i % 7) as u8)
        })
        .collect();
    let rows: Vec<Vec<u8>> = idx.chunks(w as usize).map(<[u8]>::to_vec).collect();
    let plte: Vec<u8> = pal.iter().flatten().copied().collect();
    let exp = rgba_from(
        |i| {
            let k = idx[i] as usize;
            [
                pal[k][0],
                pal[k][1],
                pal[k][2],
                trns.get(k).copied().unwrap_or(255),
            ]
        },
        n,
    );
    o.put(
        &format!("{stem}.pal8trns.png"),
        &png_by_hand(w, h, 8, 3, 1, &rows, Some(&plte), Some(&trns), false),
        w,
        h,
        "lossless",
        &exp,
    );
    // (b) 4-bit palette, 16 entries, odd width -> a padded last nibble per row
    let idx4: Vec<u8> = (0..n).map(|i| idx[i] % 16).collect();
    let rows4: Vec<Vec<u8>> = idx4
        .chunks(w as usize)
        .map(|r| {
            r.chunks(2)
                .map(|p| (p[0] << 4) | p.get(1).copied().unwrap_or(0))
                .collect()
        })
        .collect();
    let plte4: Vec<u8> = pal[..16].iter().flatten().copied().collect();
    let exp4 = rgba_from(
        |i| {
            let k = idx4[i] as usize;
            [pal[k][0], pal[k][1], pal[k][2], 255]
        },
        n,
    );
    o.put(
        &format!("{stem}.pal4.png"),
        &png_by_hand(w, h, 4, 3, 1, &rows4, Some(&plte4), None, false),
        w,
        h,
        "lossless",
        &exp4,
    );
    // (c) 1-bit gray (threshold at 128)
    let bits: Vec<u8> = g.iter().map(|&v| u8::from(v >= 128)).collect();
    let rows1: Vec<Vec<u8>> = bits
        .chunks(w as usize)
        .map(|r| {
            r.chunks(8)
                .map(|c| {
                    c.iter()
                        .enumerate()
                        .fold(0u8, |a, (k, &b)| a | (b << (7 - k)))
                })
                .collect()
        })
        .collect();
    let exp1 = rgba_from(
        |i| {
            let v = if bits[i] == 1 { 255 } else { 0 };
            [v, v, v, 255]
        },
        n,
    );
    o.put(
        &format!("{stem}.gray1.png"),
        &png_by_hand(w, h, 1, 0, 1, &rows1, None, None, false),
        w,
        h,
        "lossless",
        &exp1,
    );
    // (d) Adam7-interlaced RGBA8, filters cycled through every pass
    let rows8: Vec<Vec<u8>> = px.chunks((w * 4) as usize).map(<[u8]>::to_vec).collect();
    o.put(
        &format!("{stem}.adam7rgba8.png"),
        &png_by_hand(w, h, 8, 6, 4, &rows8, None, None, true),
        w,
        h,
        "lossless",
        &px,
    );
}

/// Chroma-subsampled JPEGs: `image` 0.25.10's encoder only writes 4:4:4, so these come from the
/// system's libjpeg-turbo `cjpeg` (an external tool, not a dependency), at q90, 4:2:0 and 4:2:2,
/// each with its partner turned 180 degrees (selah's pair gate for subsampled JPEG). Skipped
/// with a note when `cjpeg` is absent.
fn emit_cjpeg(o: &mut Out, kind: &str, w: u32, h: u32) {
    let stem = format!("{kind}-{w}x{h}");
    let px = pattern(kind, w, h);
    let rot: Vec<u8> = px.chunks(4).rev().flatten().copied().collect();
    for (tag, src) in [("", &px), (".rot180", &rot)] {
        let ppm = o.dir.join(format!("{stem}{tag}.ppm"));
        let mut data = format!("P6\n{w} {h}\n255\n").into_bytes();
        data.extend(rgb_of(src));
        std::fs::write(&ppm, &data).expect("ppm");
        for (sub, samp) in [("cj420", "2x2"), ("cj422", "2x1")] {
            let name = format!("{stem}{tag}.{sub}.q90.jpg");
            let path = o.dir.join(&name);
            let st = std::process::Command::new("cjpeg")
                .args(["-quality", "90", "-sample", samp, "-outfile"])
                .arg(&path)
                .arg(&ppm)
                .status();
            match st {
                Ok(s) if s.success() => {
                    let bytes = std::fs::read(&path).expect("read cjpeg");
                    o.put(&name, &bytes, w, h, "jpegsub", &opaque(src));
                }
                _ => println!("cjpeg unavailable: {name} skipped"),
            }
        }
        let _ = std::fs::remove_file(&ppm);
    }
}

fn decode_image(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    // soorat 1.2.0 src/texture.rs: image::load_from_memory(bytes)?.to_rgba8()
    let img = image::load_from_memory(bytes)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    Ok((img.width(), img.height(), img.into_raw()))
}

fn time_decode(path: &Path, reps: usize) -> (f64, f64) {
    let bytes = std::fs::read(path).expect("read");
    let mut t = Vec::with_capacity(reps);
    for _ in 0..reps {
        let s = Instant::now();
        let d = decode_image(&bytes).expect("decode");
        t.push(s.elapsed().as_secs_f64() * 1e3);
        std::hint::black_box(d);
    }
    t.sort_by(f64::total_cmp);
    (t[0], t[reps / 2])
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets");
    let dir = root.join("fixtures/p10");
    let out = root.join("out/p10/image");
    if std::env::var_os("P10_TIME_ONLY").is_some() {
        // timing only, against fixtures already written (used for the overflow-checks-off build)
        for f in ["photo-1024x1024.rgba8.png", "photo-1024x1024.q90.jpg"] {
            let (min, med) = time_decode(&dir.join(f), 21);
            println!(
                "time {f}: min {min:.2} ms, median {med:.2} ms (21 runs, wall clock, release)"
            );
        }
        return;
    }
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::create_dir_all(&out).expect("mkdir");
    let mut o = Out {
        dir: dir.clone(),
        list: String::new(),
    };
    emit_pattern(&mut o, "grad", 61, 37, true);
    emit_pattern(&mut o, "edges", 61, 37, true);
    emit_pattern(&mut o, "noise", 61, 37, true);
    emit_pattern(&mut o, "grad", 64, 48, true);
    emit_pattern(&mut o, "photo", 1024, 1024, false);
    for (kind, w, h) in [
        ("grad", 61, 37),
        ("edges", 61, 37),
        ("noise", 61, 37),
        ("grad", 64, 48),
        ("photo", 1024, 1024),
    ] {
        emit_cjpeg(&mut o, kind, w, h);
    }
    std::fs::write(dir.join("LIST"), &o.list).expect("list");

    for line in o.list.lines() {
        let name = line.split(' ').next().unwrap_or_default();
        let bytes = std::fs::read(dir.join(name)).expect("read");
        match decode_image(&bytes) {
            Ok((w, h, px)) => {
                std::fs::write(out.join(format!("{name}.rgba")), &px).expect("dump");
                println!("{name:32} {:7} bytes  image {w}x{h}", bytes.len());
            }
            Err(e) => println!("{name:32} image ERROR {e}"),
        }
    }
    for f in ["photo-1024x1024.rgba8.png", "photo-1024x1024.q90.jpg"] {
        let (min, med) = time_decode(&dir.join(f), 21);
        println!("time {f}: min {min:.2} ms, median {med:.2} ms (21 runs, wall clock, release)");
    }
}
