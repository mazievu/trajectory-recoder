# Spool API and static CRT evidence

## Source

Derived from the client self-contained implementation plan in this session.

## User journeys

- A client runtime can locate each spool lifecycle directory through the shared spool crate.
- A deployed Windows client does not require a Rust installation or the Microsoft VC++ runtime.

## RED and GREEN evidence

| Stage | Command | Result |
|---|---|---|
| RED | `cargo test -p spool --test test_adversarial_spool --locked` | Failed to compile because `SpoolDirectoryManager` lacked `base_path` and the six stage-directory accessors. |
| GREEN | `cargo test -p spool --test test_adversarial_spool --locked` | PASS: 5 tests. |
| Regression | `cargo test -p spool --lib --locked` | PASS: 2 tests. |
| Path compatibility | `cargo test -p supervisor uploader_companion_must_be_a_sibling_client_executable --locked` | PASS: 1 test. |
| Tier 1 | `cargo test -p tier1-feature --test test_f11_ndjson_persistence --test test_f21_session_lifecycle --test test_f22_spool_state_machine --test test_f27_spool_and_archive --locked` | PASS: 4 tests. |
| Upload pipeline | `cargo test -p uploader --test test_uploader_pipeline --locked` | PASS: 1 test, including `SESSION_ACCEPTED`. |
| Release | `cargo build --release --locked --target x86_64-pc-windows-msvc -p capture-agent -p supervisor -p uploader -p browser-host -p desktop-ui -p harness-app -p e2e-runner` | PASS with existing warnings only. |
| PE imports | `dumpbin /headers`, `/dependents`, `/imports` for all release executables | PASS: x64; no VCRUNTIME, MSVCP, CONCRT, UCRTBASE, or API-MS-WIN-CRT imports. |

## Guarantees

| What is guaranteed | Test | Type |
|---|---|---|
| The spool manager returns its root and all six lifecycle directories. | `test_spool_exposes_all_stage_directories_from_its_base_path` | Unit |
| Lifecycle transitions, error handling, retention ordering and watermarks remain covered. | `crates/spool/tests/test_adversarial_spool.rs` | Unit |
| Supervisor resolves the uploader beside itself in the standardized installation directory. | `uploader_companion_must_be_a_sibling_client_executable` | Unit |

## Coverage and remaining gates

The workspace has no configured Rust coverage target. The clean Win11 x64 VM smoke test remains required and has not run because no clean VM is available from this host; `main` must not be pushed until it passes.
