//! soorat's golden-corpus generator (port plan §4 step 4).
//!
//! Drives the frozen Rust 1.2.0 oracle in `rust-old/` (with its hidden `oracle`
//! hooks) and writes `tests/golden/<module>.tsv`, one row per call, every float
//! as its IEEE-754 bit pattern. The Cyrius harness (`tests/parity.tcyr`)
//! replays every row. Format, tokens and classes: `scripts/oracle/README.md`.
//!
//! ```sh
//! cargo run --release --locked --manifest-path scripts/oracle/Cargo.toml             # write
//! cargo run --release --locked --manifest-path scripts/oracle/Cargo.toml -- --check  # verify; exit 1 on any difference
//! ```
//!
//! Rules: the output is deterministic (grids use only + − × ÷, exact
//! conversions and a seeded xorshift; no transcendental std call, timestamp or
//! hash order); every header records the platform, and `--check` refuses a
//! corpus recorded on another; a wrong row is fixed in Rust (1.2.x) and
//! regenerated, never patched.

use std::fmt::Write as _;
use std::path::PathBuf;

mod m_color;
mod m_color_optics;
mod m_constants;
mod m_json;
mod m_layout;
mod m_lights;
mod m_lod;
mod m_math;
mod m_sprite;
mod m_tables;

/// One module's rows, written as `tests/golden/<name>.tsv`.
pub struct Module {
    pub name: &'static str,
    pub rows: Vec<String>,
}

impl Module {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            rows: Vec::new(),
        }
    }
    /// Adds a row: `fn <TAB> class <TAB> inputs <TAB> outputs <TAB> note`.
    pub fn row(&mut self, func: &str, class: char, inputs: &Toks, outputs: &Toks, note: &str) {
        let mut line = String::new();
        let _ = write!(
            line,
            "{func}\t{class}\t{}\t{}\t{}",
            inputs.text(),
            outputs.text(),
            note.replace('\t', " ")
        );
        self.rows.push(line);
    }
}

/// A space-separated token list (`-` when empty).
#[derive(Default, Clone)]
pub struct Toks(Vec<String>);

impl Toks {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    /// `g:<8 hex>`: an f32 as its bit pattern (soorat's addition to format 1).
    pub fn g(mut self, x: f32) -> Self {
        self.0.push(format!("g:{:08x}", x.to_bits()));
        self
    }
    /// A sequence of f32s: `v:<len>` then each `g:`.
    pub fn gs(mut self, xs: &[f32]) -> Self {
        self.0.push(format!("v:{}", xs.len()));
        for &x in xs {
            self = self.g(x);
        }
        self
    }
    /// `f:<16 hex>`: an f64 as its bit pattern.
    pub fn f(mut self, x: f64) -> Self {
        self.0.push(format!("f:{:016x}", x.to_bits()));
        self
    }
    /// `i:<decimal>`: an integer (or a boolean as 0 / 1).
    pub fn i(mut self, x: i128) -> Self {
        self.0.push(format!("i:{x}"));
        self
    }
    /// `s:<text>`: a string, percent-encoded: every byte outside 0x21..=0x7e, and `%`, as `%XX` (uppercase), so a
    /// token never holds a space or a tab.
    pub fn s(mut self, text: &str) -> Self {
        let mut t = String::from("s:");
        for &b in text.as_bytes() {
            if (0x21..=0x7e).contains(&b) && b != b'%' {
                t.push(b as char);
            } else {
                let _ = write!(t, "%{b:02X}");
            }
        }
        self.0.push(t);
        self
    }
    /// `n:`: `None`.
    pub fn none(mut self) -> Self {
        self.0.push("n:".to_string());
        self
    }
    /// `E:<Variant>`: `Err(RenderError::<Variant>)`, its payload not compared.
    pub fn err(mut self, variant: &str) -> Self {
        self.0.push(format!("E:{variant}"));
        self
    }
    /// `v:<len>` then each integer.
    pub fn is(mut self, xs: &[i128]) -> Self {
        self.0.push(format!("v:{}", xs.len()));
        for &x in xs {
            self = self.i(x);
        }
        self
    }
    fn text(&self) -> String {
        if self.0.is_empty() {
            "-".to_string()
        } else {
            self.0.join(" ")
        }
    }
}

/// The deterministic generator every grid draws from (the probes' xorshift64).
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// A float in about ±8 with 20 fractional bits: exact in f32 and f64.
    pub fn f(&mut self) -> f32 {
        let k = ((self.next() >> 40) as i64) - (1 << 23);
        k as f32 / 1_048_576.0
    }
    /// A positive float in (0, 16).
    pub fn pos(&mut self) -> f32 {
        let k = ((self.next() >> 40) as i64) | 1;
        k as f32 / 1_048_576.0
    }
}

/// The platform the transcendental rows depend on: glibc and the CPU class.
fn platform() -> String {
    let glibc = std::process::Command::new("ldd")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| {
            s.lines()
                .next()
                .map(|l| l.rsplit(' ').next().unwrap_or("").to_string())
        })
        .unwrap_or_else(|| "unknown".into());
    let fma = if std::is_x86_feature_detected!("fma") {
        " fma"
    } else {
        ""
    };
    format!("glibc {glibc} {}{fma}", std::env::consts::ARCH)
}

fn render(m: &Module, plat: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# soorat golden corpus, format 1: {}", m.name);
    let _ = writeln!(
        s,
        "# recorded: soorat 1.2.0 (rust-old/, oracle feature); libm: {plat}"
    );
    let _ = writeln!(
        s,
        "# regenerate: cargo run --release --locked --manifest-path scripts/oracle/Cargo.toml (never patch a row)"
    );
    let _ = writeln!(
        s,
        "# columns: fn<TAB>class<TAB>inputs<TAB>outputs<TAB>note (tokens: scripts/oracle/README.md)"
    );
    let _ = writeln!(s, "# rows: {}", m.rows.len());
    for r in &m.rows {
        s.push_str(r);
        s.push('\n');
    }
    s
}

fn main() {
    let check = std::env::args().any(|a| a == "--check");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    std::fs::create_dir_all(&root).expect("tests/golden");
    let plat = platform();

    let modules = vec![
        m_constants::module(),
        m_layout::module(),
        m_math::module(),
        m_color::module(),
        m_color_optics::module(),
        m_lights::module(),
        m_lod::module(),
        m_tables::module(),
        m_sprite::module(),
        m_json::module(),
    ];

    let mut index = String::from(
        "# soorat golden corpus: one module per line; tests/parity.tcyr replays each\n",
    );
    let mut bad = 0;
    for m in &modules {
        index.push_str(m.name);
        index.push('\n');
        let text = render(m, &plat);
        let path = root.join(format!("{}.tsv", m.name));
        if check {
            match std::fs::read_to_string(&path) {
                Ok(old) => {
                    let recorded = old.lines().nth(1).unwrap_or("");
                    if !recorded.ends_with(&plat) {
                        eprintln!(
                            "{}: recorded on another platform ({recorded}); this is {plat}",
                            path.display()
                        );
                        std::process::exit(2);
                    }
                    if old != text {
                        eprintln!("{}: differs from the oracle", path.display());
                        bad += 1;
                    }
                }
                Err(_) => {
                    eprintln!("{}: missing", path.display());
                    bad += 1;
                }
            }
        } else {
            std::fs::write(&path, &text).expect("write module");
        }
        println!("{:<10} {:>6} rows", m.name, m.rows.len());
    }
    let ipath = root.join("modules.txt");
    if check {
        if std::fs::read_to_string(&ipath).ok().as_deref() != Some(index.as_str()) {
            eprintln!("{}: differs", ipath.display());
            bad += 1;
        }
        if bad > 0 {
            std::process::exit(1);
        }
        println!("check: corpus matches the oracle ({plat})");
    } else {
        std::fs::write(&ipath, index).expect("modules.txt");
    }
}
