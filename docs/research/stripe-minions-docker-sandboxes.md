# Research: Stripe Minions & Docker Sandboxes (February 2026)

*Date: 2026-02-20*

This document surveys two developments directly relevant to CrustyClaw's architecture:
Stripe's **Minions** coding-agent system and the maturing ecosystem of
**Docker / microVM sandboxes** for AI agents.

---

## 1. Stripe Minions

### 1.1 What they are

Minions are Stripe's internal, **unattended** coding agents. Given a ticket, bug
report, or specification they produce a complete pull request — start to finish — with
no human in the loop during the coding process. All output is human-reviewed before
merge.

- **Scale:** 1,300+ PRs merged per week, all written entirely by agents.
- **Blog:** Two-part series on the Stripe Dev Blog by Alistair Gray (Leverage team):
  - [Part 1 — Feb 9, 2026](https://stripe.dev/blog/minions-stripes-one-shot-end-to-end-coding-agents)
  - [Part 2 — Feb 19, 2026](https://stripe.dev/blog/minions-stripes-one-shot-end-to-end-coding-agents-part-2)

### 1.2 Architecture

```
Slack / CLI / Web
       │
       ▼
  ┌──────────┐
  │  Minion   │ ←── Goose fork (Block open-source agent, modified for unattended use)
  │  Harness  │
  └────┬─────┘
       │
       ▼
  ┌──────────┐    ┌────────────┐
  │  Devbox   │───▶│  Toolshed  │  (centralised MCP server, ~500 tools)
  │  (EC2)    │    └────────────┘
  └────┬─────┘
       │  local lint + tests (feedback left-shift)
       ▼
  ┌──────────┐
  │    CI     │  (max 2 rounds)
  └────┬─────┘
       ▼
  Pull Request ──▶ Human Review ──▶ Merge
```

**Key components:**

| Component | Description |
|-----------|-------------|
| **Devbox** | Standardised AWS EC2 instance, pre-loaded with source code and services. Provisioned in **<10 seconds** from a warm pool. Full shell permissions — mistakes are contained to the single machine. |
| **Toolshed** | Stripe's centralised **MCP server** — "a common language for all agents at Stripe, not just minions." Exposes ~500 tools spanning internal systems and third-party SaaS. Each minion receives a curated subset. |
| **Blueprints** | Orchestration flows that mix **deterministic code nodes** with **open-ended agent loops**. Design principle: "putting LLMs into contained boxes compounds into system-wide reliability upside." |
| **Agent harness** | Fork of Block's open-source **Goose** coding agent, heavily modified for unattended operation. |
| **Feedback loops** | Local lint + test heuristics run **before** CI is invoked. Standard blueprint caps at **2 CI rounds** before returning the branch to a human. |

### 1.3 Context engine

Minions acquire context through two channels:

1. **Static context** — the devbox is pre-loaded with the full Stripe monorepo and
   services, so file reads and grep are instant.
2. **Dynamic context** — Toolshed (MCP) provides on-demand access to internal APIs,
   databases, documentation indexes, and SaaS integrations. Each task's blueprint
   specifies which tool subset is relevant, keeping the context window focused.

### 1.4 Planning and execution

Blueprints are the planning layer. They encode a directed graph of:

- **Deterministic nodes** — concrete shell commands, file transformations, API calls
  that are known ahead of time (e.g., "run linter", "create branch").
- **Agent loops** — open-ended LLM steps where the model reasons about what to do
  next given its observations (e.g., "implement the failing test case").

A blueprint terminates either when the PR is ready (CI green) or after the second
CI failure, at which point the partial work is returned to a human.

### 1.5 Human-in-the-loop

- **During coding:** No human involvement. Agents have full shell access within the
  devbox.
- **After coding:** All PRs are subject to human code review and CI before merge.
- **Escalation:** If the agent exceeds its CI budget (2 rounds) the work is surfaced
  to a human with full context.

### 1.6 Why Stripe built in-house

- Hundreds of millions of lines of mostly Ruby code.
- Heavy reliance on proprietary libraries and internal tooling.
- Infrastructure processing >$1 trillion in annual payment volume.
- General-purpose agents cannot reliably follow Stripe's internal conventions,
  compliance constraints, and deployment procedures.
- Key insight: **agents need the same context and tools as human engineers**, not a
  bolted-on integration.

### 1.7 Relevance to CrustyClaw

| Stripe concept | CrustyClaw analogue |
|----------------|---------------------|
| Devbox (EC2) | Sandbox backend (Linux NS / Apple VZ / Docker) |
| Toolshed (MCP) | Skill registry + MCP tool server |
| Blueprints | Planned: blueprint / workflow engine |
| Goose harness | CrustyClaw daemon + skill execution engine |
| Slack invocation | Signal channel |
| Human review gate | Planned: review layer with optional HITL enforcement |

---

## 2. Docker Sandboxes for AI Agents

### 2.1 Docker Sandboxes (announced Jan 30, 2026)

Docker launched **Docker Sandboxes**, purpose-built for running AI coding agents in
isolated environments.

- [Docker Sandboxes product page](https://www.docker.com/products/docker-sandboxes/)
- [Blog: A New Approach for Coding Agent Safety](https://www.docker.com/blog/docker-sandboxes-a-new-approach-for-coding-agent-safety/)
- [Blog: Run Claude Code and More Safely](https://www.docker.com/blog/docker-sandboxes-run-claude-code-and-other-coding-agents-unsupervised-but-safely/)

**Architecture:**

| Feature | Detail |
|---------|--------|
| **Isolation** | MicroVM-based (not plain containers). Each sandbox gets its own VM with a private Docker daemon. |
| **Network** | Allow / deny lists for outgoing connections. |
| **Credentials** | Network proxy intercepts outgoing API calls and swaps a sentinel value for real API keys — keys never exist inside the sandbox. |
| **Filesystem** | Workspace directory syncs at the same absolute path between host and sandbox. |
| **Management** | `docker sandbox ls` / `docker sandbox rm` (sandboxes are VMs, not containers — invisible to `docker ps`). |
| **Persistence** | Sandboxes persist until explicitly removed. Installed packages and config survive restarts. |
| **Supported agents** | Claude Code, Codex CLI, Copilot CLI, Gemini CLI, Kiro. |

**Why microVMs over containers:**
Containers share the host kernel — a kernel vulnerability in one container can
compromise all others. MicroVMs add a hard security boundary: each sandbox has its
own Linux kernel running on KVM, so escaping requires compromising both the guest
kernel and the hypervisor.

### 2.2 Docker Model Runner (GA, Feb 2026)

Docker Model Runner runs LLMs locally inside Docker with privacy guarantees.

- [Docker Model Runner docs](https://docs.docker.com/ai/model-runner/)
- [Product page](https://www.docker.com/products/model-runner/)

Key features:
- OpenAI and Ollama-compatible API endpoints.
- Inference engines: llama.cpp (all platforms), vLLM (NVIDIA + Linux/WSL2).
- Vulkan GPU support (any GPU vendor).
- OCI artifact packaging for GGUF model files.
- Integration with Dagger for local agent pipelines.

### 2.3 The broader sandbox landscape (Feb 2026)

The industry has converged on a **5-level isolation hierarchy**:

| Level | Technology | Isolation | Boot time | Use case |
|-------|-----------|-----------|-----------|----------|
| L1 | Containers (Docker, Podman) | Linux namespaces + cgroups | ~100 ms | Trusted internal code |
| L2 | User-space kernels (gVisor) | Syscall interception | ~200 ms | Moderate risk |
| L3 | Micro-VMs (Firecracker, Kata, libkrun) | Full KVM guest kernel | ~125-200 ms | **Untrusted / LLM-generated code** |
| L4 | Library OS (Microsoft LiteBox) | Minimal OS primitives | Experimental | Research / future |
| L5 | Confidential computing (SEV-SNP, TDX) | Hardware-encrypted memory | VM boot | Secrets-sensitive workloads |

**Notable platforms:**

| Platform | Isolation | Key differentiator |
|----------|-----------|-------------------|
| **E2B** | Firecracker microVMs | Purpose-built for AI agents. ~150 ms boot, <5 MiB overhead, 150 VMs/sec/host. Python/JS SDKs. Open-source. 24-hour session limit. CPU only. |
| **Docker Sandboxes** | MicroVMs | Integrated into Docker Desktop. Credential proxy. First-class support for major coding agents. |
| **Daytona** | Docker/Seccomp or Kata Containers | Multiple isolation tiers. Stateful workspaces. |
| **Modal** | gVisor | Optimised for Python ML workloads. GPU support. |
| **Sprites.dev** | Firecracker | Launched Jan 2026. Stateful sandboxes with checkpoint/rollback on NVMe. |
| **Northflank** | Firecracker | Managed microVM platform. Self-serve scaling. |

**Industry consensus (Feb 2026):**
> "The execution environment is now as critical as the model itself. The winners in
> 2026 aren't just picking the best LLM — they're building robust, secure
> infrastructure that can safely execute whatever the model generates."
>
> AWS built Firecracker for Lambda. Google built gVisor for Search and Gmail. Azure
> uses Hyper-V for ephemeral agent sandboxes. Every major cloud pointed their
> strongest isolation primitive at AI.

---

## 3. Implications for CrustyClaw

### 3.1 Sandboxing

CrustyClaw's existing `SandboxBackend` trait (Apple VZ, Linux NS, Noop) aligns with
the L1-L3 isolation levels. Next steps:

- **Docker Sandbox integration** — add a `DockerSandboxBackend` that uses `docker sandbox`
  commands for microVM-level isolation with credential proxying.
- **Firecracker backend** — for self-hosted deployments where Docker Desktop is not
  available.
- **Isolation level selection** — let the policy engine choose isolation level based
  on skill trust level (internal = L1, untrusted/LLM-generated = L3).

### 3.2 Context engine

Stripe's Toolshed pattern validates CrustyClaw's MCP-based approach:

- Centralised tool server exposing curated tool subsets per task.
- Static context (local codebase) + dynamic context (MCP tools) as separate channels.
- Per-task tool scoping to manage context window size.

### 3.3 Blueprint / planning layer

CrustyClaw should adopt the blueprint pattern:

- Directed graphs mixing deterministic steps with LLM-driven agent loops.
- Configurable CI budget (max rounds before escalation).
- Local pre-flight checks (lint, test heuristics) before expensive CI runs.

### 3.4 Review and feedback loop

- Automated review gates: lint, type-check, test pass.
- Optional human-in-the-loop enforcement at configurable checkpoints.
- Escalation policy when agent exceeds its execution budget.
- Config-driven: operators choose where human approval is required.

---

## Sources

- [Minions: Stripe's one-shot, end-to-end coding agents (Part 1)](https://stripe.dev/blog/minions-stripes-one-shot-end-to-end-coding-agents)
- [Minions: Part 2](https://stripe.dev/blog/minions-stripes-one-shot-end-to-end-coding-agents-part-2)
- [Stripe's AI agents now write 1,000+ PRs/week — Medium](https://medium.com/reading-sh/stripes-ai-agents-now-write-1-000-pull-requests-per-week-cb0b063538f7)
- [Stripe's Autonomous Coding Agents Generate Over 1,300 PRs/Week — Analytics India Magazine](https://analyticsindiamag.com/ai-news/stripes-autonomous-coding-agents-generate-over-1300-prs-a-week)
- [Docker Sandboxes: A New Approach for Coding Agent Safety](https://www.docker.com/blog/docker-sandboxes-a-new-approach-for-coding-agent-safety/)
- [Docker Sandboxes: Run Claude Code and More Safely](https://www.docker.com/blog/docker-sandboxes-run-claude-code-and-other-coding-agents-unsupervised-but-safely/)
- [Docker Sandboxes product page](https://www.docker.com/products/docker-sandboxes/)
- [Docker Model Runner docs](https://docs.docker.com/ai/model-runner/)
- [Docker Model Runner product page](https://www.docker.com/products/model-runner/)
- [How to sandbox AI agents in 2026 — Northflank](https://northflank.com/blog/how-to-sandbox-ai-agents)
- [Best code execution sandbox for AI agents in 2026 — Northflank](https://northflank.com/blog/best-code-execution-sandbox-for-ai-agents)
- [Agent Sandboxes: A Practical Guide — vietanh.dev](https://www.vietanh.dev/blog/2026-02-02-agent-sandboxes)
- [AI agent sandboxing guide — Substack](https://manveerc.substack.com/p/ai-agent-sandboxing-guide)
- [E2B — The Enterprise AI Agent Cloud](https://e2b.dev/)
- [Daytona vs E2B in 2026 — Northflank](https://northflank.com/blog/daytona-vs-e2b-ai-code-execution-sandboxes)
- [Top AI sandbox platforms in 2026 — Northflank](https://northflank.com/blog/top-ai-sandbox-platforms-for-code-execution)
- [Introducing Docker Model Runner](https://www.docker.com/blog/introducing-docker-model-runner/)
- [Run NanoClaw in Docker Shell Sandboxes](https://www.docker.com/blog/run-nanoclaw-in-docker-shell-sandboxes/)
