//! Fuzz target for the TOML configuration parser.
//!
//! Run with: cargo +nightly fuzz run fuzz_config_parser
//!
//! This exercises `AppConfig::parse()` with arbitrary byte sequences to find
//! panics, hangs, or memory issues in the TOML parsing and validation pipeline.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Try to parse arbitrary bytes as a TOML config
    if let Ok(s) = std::str::from_utf8(data) {
        // We don't care about the result — just that it doesn't panic
        let _ = crustyclaw_config::AppConfig::parse(s);
    }
});
