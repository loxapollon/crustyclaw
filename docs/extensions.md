# Extension Guide

CrustyClaw extends functionality through Forgejo Actions — plugins run as
sandboxed CI/CD workflows. This guide covers how to write, register, and
deploy extensions.

## Overview

Extensions (skills) are self-contained units of work that the daemon executes
in response to messages. Each skill:

1. Matches incoming messages by pattern or explicit invocation
2. Executes inside a sandbox (configurable isolation backend)
3. Returns a string response routed back through the message bus

## Writing a skill

Implement the `Skill` trait from `crustyclaw-core`:

```rust
use crustyclaw_core::{BoxFuture, skill::{Skill, SkillError}};
use crustyclaw_core::message::Envelope;

struct EchoSkill;

impl Skill for EchoSkill {
    fn name(&self) -> &str { "echo" }
    fn description(&self) -> &str { "Echoes the message back" }

    fn execute(&self, message: &Envelope) -> BoxFuture<'_, Result<String, SkillError>> {
        let body = message.body.clone();
        Box::pin(async move { Ok(body) })
    }
}
```

The `execute` method returns a `BoxFuture` because the `Skill` trait is used
with dynamic dispatch (`dyn Skill`). Inside the boxed future you have full
async capability.

## Registering a skill

Add skills to the `SkillRegistry`:

```rust
use crustyclaw_core::skill::SkillRegistry;

let mut registry = SkillRegistry::new();
registry.register(Box::new(EchoSkill));
```

## Isolated skills

For skills that should execute inside a sandbox, use `IsolatedSkill`:

```rust
use crustyclaw_core::skill::IsolatedSkill;
use crustyclaw_core::isolation::{SandboxConfig, select_backend, BackendPreference};

let backend = select_backend(&BackendPreference::Auto);
let config = SandboxConfig::builder("my-skill")
    .memory_bytes(128 * 1024 * 1024)
    .cpu_fraction(0.25)
    .build()
    .unwrap();

let skill = IsolatedSkill::new("my-skill", "Runs in a sandbox", backend, config);
```

## Forgejo Action plugins

For declarative plugin registration, use the `ActionPlugin` derive macro:

```rust
use crustyclaw_macros::ActionPlugin;

#[derive(ActionPlugin)]
#[action_plugin(
    name = "weather",
    version = "0.1.0",
    description = "Fetches weather forecasts"
)]
struct WeatherPlugin;
```

### Action hooks

Register hook functions that fire on specific events:

```rust
use crustyclaw_macros::action_hook;

#[action_hook(event = "message.received", priority = 10)]
fn on_message_received() {
    // Handle the event
}
```

### Security policies (declarative)

Define compile-time security policies with the `security_policy!` macro:

```rust
use crustyclaw_macros::security_policy;

security_policy! {
    allow "admin" => "read" on "config",
    deny  "guest" => "write" on "secrets",
}
```

## Sandbox configuration

See [configuration.md](configuration.md#isolation) for the full isolation
config reference. Key parameters:

- `default_memory_bytes` — memory limit per sandbox
- `default_cpu_fraction` — CPU allocation
- `default_timeout_secs` — execution timeout
- `default_network` — network access policy
- `max_concurrent` — concurrent sandbox limit
