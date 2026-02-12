# CrustyClaw Roadmap: Research & Implementation

## Phase 0 — Foundation (Current)
- [x] Initialize repository with README.md and CLAUDE.md
- [x] Research Rust metaprogramming landscape
- [ ] Decide on metaprogramming strategy for the project
- [ ] Set up Cargo workspace structure

## Phase 1 — Core Scaffolding
- [ ] Create `Cargo.toml` with workspace layout
- [ ] Create `src/main.rs` entry point with CLI skeleton (`clap`)
- [ ] Create `crustyclaw-macros` proc-macro crate for derive/attribute macros
- [ ] Set up CI via Forgejo Actions (`.forgejo/workflows/ci.yml`)
- [ ] Add `cargo-audit`, `clippy`, `fmt` checks to CI
- [ ] Add `#![deny(unsafe_code)]` policy

## Phase 2 — Security Primitives (Metaprogramming-Heavy)
- [ ] Implement `#[derive(Validate)]` — compile-time input validation from annotations
- [ ] Implement `#[derive(Redact)]` — auto-redact sensitive fields in Debug/Display
- [ ] Implement `#[derive(SecureZeroize)]` — zeroize sensitive memory on Drop
- [ ] Implement type-state pattern for session lifecycle (Unauth → Auth → Authorized)
- [ ] Add `const` assertions for security policy invariants (key lengths, TLS versions)
- [ ] Build script: embed git commit hash, build timestamp, lockfile checksum

## Phase 3 — Configuration & Policy Engine
- [ ] Design config format (TOML/YAML) with `serde` + `Validate` derive stacking
- [ ] Implement security policy DSL via function-like proc macro (`security_policy!{}`)
- [ ] Compile-time policy validation (role/action/resource well-formedness)
- [ ] Runtime policy evaluation engine with zero-cost compiled match trees

## Phase 4 — Forgejo Actions Extension System
- [ ] Define `ActionPlugin` trait
- [ ] Implement `#[derive(ActionPlugin)]` — generates input parsing, output setters, metadata
- [ ] Implement `#[action_hook(event, priority)]` attribute macro for hook registration
- [ ] Build-script: parse `action.yml` → generate typed Rust bindings, validate schema
- [ ] Plugin discovery via `inventory`/`linkme` crate pattern
- [ ] `workflow_step!{}` macro for compile-time workflow fragment validation
- [ ] Integration test harness via `action_integration_test!` declarative macro

## Phase 5 — Platform & Hardening
- [ ] Platform-conditional sandboxing (`seccomp`/`landlock` on Linux, fallback elsewhere)
- [ ] Supply-chain: `cargo-vet` or `cargo-crev` integration
- [ ] Dependency pinning and reproducible builds
- [ ] Fuzz testing harness (`cargo-fuzz` / `afl`)
- [ ] SBOM generation in CI

## Phase 6 — Documentation & Release
- [ ] Crate-level and public-API documentation (`cargo doc`)
- [ ] User-facing CLI documentation
- [ ] Extension authoring guide (how to write a Forgejo Action plugin)
- [ ] Publish to crates.io (when ready)
- [ ] Versioned releases via Forgejo Actions

---

## Metaprogramming Strategy Summary

| Technique | Where Used | Priority |
|-----------|-----------|----------|
| `macro_rules!` | Test harnesses, CLI boilerplate, repetitive patterns | Medium |
| Derive macros | Validate, Redact, SecureZeroize, ActionPlugin | **High** |
| Attribute macros | `#[action_hook]`, extension registration | **High** |
| Function-like proc macros | `security_policy!{}`, `workflow_step!{}` DSLs | Medium |
| `const fn` + const generics | Security invariant assertions, sized buffers | Medium |
| Type-state patterns | Session lifecycle, build pipeline states | **High** |
| Build scripts | Git metadata, action.yml codegen, lockfile checks | Medium |
