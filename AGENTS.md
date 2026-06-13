# AGENTS.md — Instructions for AI agents

## Project

Real-time air-defense system (EDTH hackathon): N interceptors publish their detections on a
**NATS** broker, and a central **decision engine** fuses tracks, prioritizes threats and
assigns targets. See `README.md` for the full context and the whole-backend map.

## Architecture — golden rule

**Platform → NATS → decision engine → NATS → Platform.** Interceptors never talk to each
other directly and do not decide their own targets: all assignment logic lives in the
central decider. In the live demo that decider is `vanguard-control`
(`vanguard-control/src/engagement.rs`); in the modular event-bus design it is
`vanguard-orchestrator` (`vanguard-orchestrator/src/orchestrator.rs`). Do not push targeting
decisions down to the platform/interceptor side.

## Workspace structure

See the table and diagram in `README.md` for the authoritative layout. In short:

| Crate | Role |
|---|---|
| `vanguard-core` | Shared serde types and NATS subjects exchanged on the bus |
| `vanguard-map` | Ground-truth simulation (demo) |
| `vanguard-control` | Platform host + engagement engine — the central decider (demo) |
| `vanguard-orchestrator` / `vanguard-platform` / `vanguard-interceptor` | Modular event-bus runtime |

- Types that travel over NATS (`InterceptorReport`, `InterceptorOrder`, `DetectedThreat`, `Position`) must live in `vanguard-core` with `#[derive(Serialize, Deserialize)]`, not be duplicated in each binary.
- ⚠️ Known TODO: an empty `common/` crate still exists from an earlier migration plan; the shared types currently live in `vanguard-core`.

## Code conventions

- **Minimalism first**: always the simplest solution with the fewest lines of code. No speculative abstraction (trait, generic, module) until there are at least two concrete uses; no handling of cases that do not happen yet; no extra dependency if the standard library suffices. Hackathon context: short code that works beats extensible code.
- Rust edition 2024, async with **tokio** (`#[tokio::main]`, `rt-multi-thread`).
- Messaging via **async-nats** + **serde_json**. Name NATS subjects hierarchically: `platform.<id>.report`, `control.*` (or a consistent equivalent — check what exists before creating a new one).
- The decision engine runs on a **tick** (target: recompute every 1–2 s): keep the pure separation (state + computation) from IO (NATS) — the assignment logic must stay testable without a broker.
- Only emit an order when it **changes** — do not spam the bus.
- No `unwrap()` on IO/network paths; reserve `unwrap`/`expect` for internal invariants.

## Commands

```bash
cargo build                 # build the whole workspace
cargo run -p vanguard-map   # ground-truth map
cargo run -p vanguard-control  # platform host + engagement engine
cargo test                  # tests
cargo clippy --workspace    # lint — fix any warnings you introduce
cargo fmt                   # format before committing
```

Local NATS: `docker run -p 4222:4222 nats:latest` (default port 4222).

## Planned algorithmic directions

If asked to improve the assignment, the methods targeted by the challenge are: Hungarian
algorithm / max-flow (optionally via OR-Tools), Kalman filtering for track fusion, and
accounting for real constraints (range `reach`, ammunition `ammo`, reload time, engagement
probability). The Hungarian assignment with hysteresis is implemented in
`vanguard-control`; Kalman fusion exists in `vanguard-core` but is not yet wired into the
demo.

## Git

- Short, imperative, English commit messages (consistent with the existing history).
- No `Co-Authored-By` line in commits.
