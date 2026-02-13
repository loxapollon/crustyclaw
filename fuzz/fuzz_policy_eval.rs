//! Fuzz target for the policy engine evaluation.
//!
//! Run with: cargo +nightly fuzz run fuzz_policy_eval
//!
//! Exercises the policy engine with arbitrary role/action/resource strings
//! to find panics or incorrect behavior.

#![no_main]

use crustyclaw_config::policy::{PolicyEngine, PolicyRule};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 6 {
        return;
    }

    // Use first 2 bytes as split points to divide data into 3 strings
    let split1 = (data[0] as usize % (data.len() - 1)).max(1);
    let split2 = (data[1] as usize % (data.len() - split1)).max(1) + split1;

    let role = std::str::from_utf8(&data[2..split1]).unwrap_or("user");
    let action = std::str::from_utf8(&data[split1..split2]).unwrap_or("read");
    let resource = std::str::from_utf8(&data[split2..]).unwrap_or("data");

    let mut engine = PolicyEngine::new();
    engine.add_rule(PolicyRule::allow("admin", "*", "*"));
    engine.add_rule(PolicyRule::deny("*", "write", "secrets"));
    engine.add_rule(PolicyRule::allow("user", "read", "*"));

    // Should never panic regardless of input
    let _ = engine.evaluate(role, action, resource);
    let _ = engine.is_allowed(role, action, resource);
});
