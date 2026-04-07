# AGENTS.md

Guidance for AI coding agents (Cursor, Claude Code, etc.) and humans working on **just-bash**: day-to-day development, architecture, and **incremental refactors** that shrink in-tree code and **use more crates** without dropping behavior the project already supports—**no regressions, no silent behavior changes, no cut features, and no “test bending”** (see **Behavioral contract** below).

## Project overview

just-bash is a sandboxed Bash interpreter written in Rust, aimed at AI agents and automated script execution. It provides a bash frontend (`brush-parser`), interpreter, virtual in-memory filesystem, many Unix-style commands, network controls, and (with the `sandbox` feature) a Vercel-compatible Sandbox API.

## Build and development

```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo test                     # Full test run (675+ unit tests)
cargo nextest run -p just-bash # Faster test runner for this crate
cargo test <test_name>         # Run a specific test by name
cargo test -- --nocapture      # Show stdout during tests
cargo fmt                      # Format
cargo clippy                   # Lint
cargo run --bin loc-heatmap    # LOC heatmap (src/ + tests/) to stdout; redirect to scripts/LOC_HEATMAP.txt to snapshot
```

## Keeping this guide current

**Proactively update `AGENTS.md` in the same change** when you introduce something that affects how people or agents work on the repo day to day—not for every small fix, but whenever the update would prevent stale instructions or wrong assumptions.

Typical triggers: new or removed **Cargo features** / **bins** / **workspace layout**; renamed or moved **module boundaries** from this document; new **required workflows** (CI, fmt/clippy/test expectations); new **development tools** (scripts, heatmaps, codegen); materially different **command registration** or **public API** surfaces called out here.

If a limitation belongs in [`KNOWN_LIMITATIONS.md`](KNOWN_LIMITATIONS.md), update that file too; keep **this** file focused on *how to work on* just-bash.

## Architecture

The crate exposes a library (`just_bash`) and, with the `cli` feature, the `just-bash` binary.

### Execution pipeline

```
Input → parser::parse() (brush-parser) → AST → ExecutionEngine::execute_script() → Result
```

### Module map

- **`parser`** (`src/parser.rs`) — Thin wrapper around **`brush-parser`**: tokenization, parsing, and AST types (re-exported from `lib.rs` as `just_bash::ast`, `ParseError`, etc.).
- **`interpreter/`** — Execution engine (largest area): `execution_engine.rs`, `word_expansion.rs`, `arithmetic.rs`, `pipeline_execution.rs`, `control_flow.rs`, `builtin_dispatch.rs`, `redirections.rs`, plus `expansion/` (parameter, brace, tilde, command substitution, …), `helpers/` (tests, conditions, file tests, …), and `builtins/`.
- **`commands/`** — External commands implementing the async `Command` trait; register in `commands/mod.rs`. Several delegate to crates (`sed-rs`, `awk-rs`, `jaq-all`, …); `curl/` and shared `jaq_support` follow the adapter pattern.
- **`fs/`** — Virtual filesystem (`InMemoryFs`, paths, types). No real host FS for normal operation.
- **`sandbox/`** — Optional (`feature = "sandbox"`): execution limits (recursion, command count, loop iterations) and Sandbox API types.
- **`network/`** — URL allow-list, HTTP policy, fetching (`ureq` integration).
- **`shell/`** — Glob expansion including extended globs (`@()`, `*()`, …).
- **`bash.rs`** — High-level `Bash` API for driving the interpreter.

### Key design decisions

- **Async:** Tokio; `block_in_place` bridges synchronous interpreter paths where needed.
- **No real filesystem** for user paths: use `InMemoryFs` unless a **documented** crate limitation escapes to the host ([`KNOWN_LIMITATIONS.md`](KNOWN_LIMITATIONS.md)).
- **Execution limits** (sandbox feature): cap recursion depth, commands, and loop iterations.
- **Network allow-list:** outbound HTTP must pass validation.
- **`InterpreterState`:** central state (variables, functions, options, context) threaded through the interpreter.
- **Standalone:** Keep the crate generic and publishable. Do not couple to `icloud-*` or other product-specific crates.

### Known limitations (crates and integrations)

See [`KNOWN_LIMITATIONS.md`](KNOWN_LIMITATIONS.md). Each entry should tie to an `#[ignore]` test. [`KNOWN_ISSUES.md`](KNOWN_ISSUES.md) redirects there.

### Adding a new command

Implement the async `Command` trait in `src/commands/mod.rs`, register in the command dispatch table, and follow patterns under `src/commands/`.

---

## Refactor north star

1. **Less custom code** — Prefer maintained dependencies when semantics, security, and tests align.
2. **Clearer architecture** — Keep parse, interpret/expand, virtual FS, commands, and network boundaries explicit; adapters stay thin.
3. **No silent regressions** — `cargo test` stays green except where a limitation is documented with an `#[ignore]` test.
4. **Standalone crate** — No coupling to product-specific stacks.

## Behavioral contract (no regressions, no test bending)

Treat **existing behavior** as the product contract: what the interpreter and commands already do today, as enforced by **`cargo test`** and described for POSIX/bash/GNU-aligned tools, must **stay** unless the project explicitly decides on a breaking change (rare) and documents it.

- **No regressions** — Observable outcomes (stdout, stderr, exit status, errors, sandbox boundaries) must not worsen or drift accidentally.
- **No unannounced behavior changes** — Refactors are **implementation** changes; user-visible semantics should remain the same. If something must change, call it out in the change description and update tests/docs deliberately—not as a side effect.
- **No cut features** — Do not remove flags, code paths, or capabilities the tree already supports to “simplify” or make tests pass. Narrowing scope belongs only in a **documented** decision, with [`KNOWN_LIMITATIONS.md`](KNOWN_LIMITATIONS.md) and `#[ignore]` where crates block full parity.
- **Do not bend tests** — Never weaken assertions, replace precise expectations with vague `contains`/`||` alternatives, delete tests, or mark tests `#[ignore]` **just to go green** while leaving wrong behavior in place. Fix the implementation (or document a real upstream limitation and tie it to `KNOWN_LIMITATIONS.md` + `#[ignore]` with tests that still state the **desired** correct behavior). Slipping bugs under looser tests is forbidden.

## Measuring “code reduction” (report honestly)

When someone asks whether the crate got smaller, **quote numbers that match the question**.

### Whole crate (default for “did we reduce LOC?”)

Count **all Rust sources that ship with the repo**, including **integration tests under `tests/`**, **`src/bin/`**, and **`#[cfg(test)]` modules still in `src/`**, excluding build artifacts:

```bash
find . -path ./target -prune -o -name '*.rs' -print | xargs wc -l | tail -1
```

That total is the fair answer to “across this whole crate including tests, did we drop lines?” **Moving** a large `#[cfg(test)]` module from `src/commands/foo/tests.rs` to `tests/foo.rs` changes layout and what gets compiled when, but it **does not delete** lines—it mostly moves them. If you only report `find src -name '*.rs' | xargs wc -l`, you can show a big `src/` drop while **total Rust is flat or up**. Do not present that as “the crate shrank” without saying it was restructuring.

If you publish a before/after, note explicitly:

- **Total Rust** (command above), and optionally
- **`src/` only** (implementation + embedded tests), and
- **`tests/` only** (integration tests).

### What counts as real reduction

- **Real:** deleting duplicate logic, delegating to a crate, collapsing copy-paste, removing dead code—with the **same** tests and behavior.
- **Not real reduction (still valuable):** relocating tests, splitting modules, or moving code between `src/` and `tests/` without deleting lines. Call that **reorganization** or **clearer boundaries**.

### `loc-heatmap`

The `loc-heatmap` binary breaks down LOC by area; use it when you need *where* lines live, not as a substitute for **total Rust** when the goal is whole-crate shrinkage.

### Tests for delegating commands (`sed`, `awk`, …)

Do **not** mirror the upstream crate’s full language or feature test matrix inside just-bash. Keep tests that prove **this crate’s** wiring: CLI parsing, **`InMemoryFs`** reads (including `-f` script paths), stdin / `-`, error forwarding, and **`#[ignore]`** cases listed in [`KNOWN_LIMITATIONS.md`](KNOWN_LIMITATIONS.md). Defer semantics coverage to **sed-rs** / **awk-rs** (or forks), not duplicated hundreds of lines here.

## How to refactor incrementally

Work in **small, reviewable steps**. Each change should:

- Preserve or improve **observable** behavior (stdout/stderr, exit status, tests)—see **Behavioral contract (no regressions, no test bending)**.
- Prefer **wrappers and adapters** over wide rewrites.
- Keep **async/sync** boundaries intentional; match existing `interpreter/` and `commands/` patterns.
- Run **`cargo fmt`**, **`cargo clippy`**, and **`cargo test`** before calling the step done.

When a crate cannot honor the virtual FS, sandbox env, or bash semantics, **document it** under [`KNOWN_LIMITATIONS.md`](KNOWN_LIMITATIONS.md) and keep an `#[ignore]` test.

### Suggested order of attack

1. **Commands** — Keep delegating to crates; keep path and env wiring on `InMemoryFs` / `CommandContext`.
2. **Interpreter “leaf” logic** — Crate candidates only with bash parity and tests.
3. **Parsing** — Extend or fork **`brush-parser`**; do not grow a second full parser in-tree.
4. **Networking** — Crates for URL/HTTP primitives; policy stays in `network/`.
5. **Globbing** — Replace hand-rolled pieces only with tests proving bash parity for supported constructs.

Avoid large drive-by rewrites of **`interpreter/`** core execution without coverage.

## Crate adoption checklist

| Question | If “no” or “maybe” |
|----------|-------------------|
| **User-visible file I/O** only via abstractions you control (`InMemoryFs`, buffers)? | Document in `KNOWN_LIMITATIONS.md` or plan adapter/fork. |
| **Environment** must reflect sandbox `CommandContext.env`? | Same. |
| **License** and maintenance OK? | Escalate. |
| **Tests** cover your integration path? | Add tests; do not rely only on upstream crate tests. |

## Technical debt from crates: `KNOWN_LIMITATIONS.md`

When you adopt a dependency that cannot meet sandbox or bash semantics, **update** [`KNOWN_LIMITATIONS.md`](KNOWN_LIMITATIONS.md) using this template:

```markdown
## <short title> (<crate-name> <version>)

**Symptom:** …

**Root cause:** … (e.g. no virtual FS hook / uses `std::env` / ignores `ORS`)

**Impact:** … (user-visible; security boundary if any)

**Tracked by:** `path::to::test_name` (`#[ignore]`)

**Workaround (if any):** …

**Resolution:** Prefer upstream fix; else **vendored fork** or **fork**; link issue/PR when filed.
```

### Forking and vendoring

- Say **why** upstream is not enough (API, timeline, scope).
- Keep patches **minimal** (FS trait, env injection, behavior fixes).
- Use **`[patch]` / git deps / subtree** as the project standard; avoid unrelated style churn.

## What *not* to do

- **Bend tests or drop coverage** to hide regressions (see **Behavioral contract** above). The only acceptable way to “defer” behavior is a **documented** limitation with `#[ignore]` tests that still encode the **target** semantics.
- Reimplement large subsystems in-tree when a crate plus adapter suffices.
- Couple **just-bash** to non-generic product code.

---

*Refactor guidance here targets **incremental**, **safe** crate adoption. Large rewrites are out of scope unless explicitly requested.*
