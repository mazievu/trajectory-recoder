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

## Guarantees

| What is guaranteed | Test | Type |
|---|---|---|
| The spool manager returns its root and all six lifecycle directories. | `test_spool_exposes_all_stage_directories_from_its_base_path` | Unit |
| Lifecycle transitions, error handling, retention ordering and watermarks remain covered. | `crates/spool/tests/test_adversarial_spool.rs` | Unit |
| Supervisor resolves the uploader beside itself in the standardized installation directory. | `uploader_companion_must_be_a_sibling_client_executable` | Unit |

## Coverage and remaining gates

The workspace has no configured Rust coverage target. Release build/import checks, Tier 1 tests, and the required clean Win11 VM smoke test are recorded after they run; the VM gate must pass before pushing `main`.
