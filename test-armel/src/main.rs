fn main() {
    println!("NASty test binary");
    println!("  arch: {}", std::env::consts::ARCH);
    println!("  os: {}", std::env::consts::OS);
    println!("  family: {}", std::env::consts::FAMILY);

    // Test basic file I/O
    match std::fs::read_to_string("/proc/cpuinfo") {
        Ok(s) => {
            let first_line = s.lines().next().unwrap_or("(empty)");
            println!("  cpu: {}", first_line.trim());
        }
        Err(e) => println!("  cpu: error reading /proc/cpuinfo: {}", e),
    }

    println!("OK - toolchain works on this platform");
}
