# Known limitations (crate-induced and integrations)

This file is the **canonical registry** for behavior gaps introduced by **external crates** or integrations where full bash / sandbox parity is not yet achievable. When you adopt or deepen use of a dependency and it cannot honor `InMemoryFs`, sandbox environment, or POSIX/bash semantics, **document it here** and add or keep an `#[ignore]` test.

**Resolution path** is almost always: **upstream fix or API** → if blocked, **narrow fork / vendored patch** (document patch focus).

---

## awk (`awk-rs` 0.1)

**Symptom:** Some awk features diverge from expected bash/POSIX behavior in this environment.

### ORS (Output Record Separator) ignored

**Root cause:** `awk-rs` 0.1 does not honour custom `ORS`; e.g. `ORS="|"` still yields newline-separated records. `OFS` works.

**Impact:** User scripts depending on `ORS` for formatting get wrong output.

**Tracked by:** `commands::awk::tests::test_ors_known_issue` (`#[ignore]`)

**Workaround:** Post-process output in the shell or avoid custom `ORS`.

**Resolution:** Upstream `awk-rs` ORS support; else **fork** with ORS handling aligned with the crate’s record emission path.

### `ENVIRON` reads host process environment, not sandbox env

**Root cause:** `awk-rs` fills `ENVIRON` from `std::env` at runtime. There is no API to inject the interpreter’s `CommandContext.env`.

**Impact:** `ENVIRON` in awk does not reflect the sandboxed environment; can leak host env view or confuse scripts.

**Tracked by:** `commands::awk::tests::test_environ_sandbox_known_issue` (`#[ignore]`)

**Workaround:** Pass values as awk variables/` -v` where possible instead of `ENVIRON`.

**Resolution:** Upstream hook for env map; else **fork** exposing `ENVIRON` from a provided map.

---

## sed (`sed-rs` 1.0.0)

**Symptom:** Certain sed commands perform real filesystem I/O instead of using the virtual fs.

### `r` / `w` / `R` / `W` use the host filesystem

**Root cause:** `sed-rs` uses `std::fs` internally for read/write file commands.

**Impact:** Paths typically do not exist on the host, so `r` yields no input; `w` may fail or **write to the real disk**—bypassing `InMemoryFs` and sandbox expectations.

**Tracked by:**

- `commands::sed::tests::test_read_file_command` (`#[ignore]`)
- `commands::sed::tests::test_write_file_command` (`#[ignore]`)

**Workaround:** Avoid `r`/`w`/`R`/`W` against paths that must stay in the sandbox; use pipelines and temp files **inside** the virtual fs only if the crate is fixed—today those commands are unsafe for sandbox purity.

**Resolution:** Upstream **virtual FS trait** or callbacks for file ops; else **fork** replacing `std::fs` with adapters to `InMemoryFs` (or buffers).

---

## Adding a new entry

Use the template in [`AGENTS.md`](AGENTS.md) (section *Technical debt from crates*). Every entry should name a **test** (`#[ignore]`) that defines the expected behavior once the limitation is lifted.
