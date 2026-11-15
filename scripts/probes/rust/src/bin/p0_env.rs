//! P0: record the probe environment (glibc, CPU features) for every corpus header.
fn main() {
    let glibc = std::process::Command::new("ldd")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("").to_string())
        .unwrap_or_default();
    println!("glibc\t{glibc}");
    println!("fma\t{}", std::is_x86_feature_detected!("fma"));
    println!("avx2\t{}", std::is_x86_feature_detected!("avx2"));
    println!("soorat_oracle_sin1\t{:08x}", 1.0f32.sin().to_bits());
}
