# Client Runtime Safety — TDD Evidence

Source plan: derived during this implementation run.

## User journeys

1. As an enrolled client, I want the capture process to refuse missing machine/user identity so session data cannot collide across workstations.
2. As an administrator, I want a background uploader to use only an explicit, safe collector endpoint and its client role, so it cannot send data to an accidental destination or start in server mode.
3. As a client, I want the device credential stored with Windows DPAPI and heartbeat/upload authentication, so the server can safely track presence and associate uploads with the enrolled machine.
4. As a client service, I want Session 0 Supervisor to start and stop only its headless uploader companion, so the uploader remains managed without attempting to capture an interactive desktop from Session 0.
5. As a client deployment, I want every client process to use the same explicit `client.env`, so server-role environment variables and relative spool paths cannot change capture or upload behaviour.

## RED / GREEN

| Guarantee | Test target | RED evidence | GREEN evidence |
|---|---|---|---|
| Capture identity requires enrolled machine and user IDs | `cargo test -p capture-agent runtime_identity_requires_enrolled_machine_and_user_ids` | Failed to compile because `RuntimeIdentity::from_values` did not exist. | Passed after runtime identity resolver replaced hardcoded IDs. |
| Uploader fails closed on endpoint/role policy | `cargo test -p uploader tests::` | Initial uploader target was blocked by an unrelated in-progress server compile error; the new tests referenced missing `ClientRuntimeConfig` and `resolve_runtime_role`. | 5 policy tests passed after the resolver implementation. |
| Credential protection and authenticated upload pipeline work | `cargo test -p uploader` | Pipeline initially received `401` because it was not registered; then exposed duplicate JWT response aliases. | 7 uploader unit tests and the registered end-to-end upload test passed. |
| Supervisor accepts only client uploader companion | `cargo test -p supervisor uploader_companion_must_be_a_sibling_client_executable` | Failed to compile because companion path and role validators did not exist. | Passed after the Session 0 child lifecycle implementation. |
| Capture rejects a server role, relative spool path, and uses the configured spool | `cargo test -p capture-agent capture_runtime_rejects_non_client_role_and_uses_configured_spool` | The new relative-spool assertion failed against the permissive loader. | Passed after absolute `SPOOL_DIR` validation was added. |
| Supervisor reads an explicit file rather than machine environment | `cargo test -p supervisor supervisor_loads_client_config_from_an_explicit_file` | Failed to compile because `config::ClientRuntimeConfig` did not exist. | Passed after `--config` and service-path loading were added. |

## Final validation

- `cargo test -p uploader` — PASS: 7 unit tests, 1 integration test.
- `cargo test -p capture-agent` — PASS: 3 unit tests.
- `cargo test -p supervisor` — PASS: 8 tests, including child-process reaping on graceful shutdown.
- `cargo test -p config` — PASS: 6 existing configuration tests.
- PowerShell parser — PASS: `deployment/Install-InteractiveCaptureTask.ps1` has no syntax errors.

Known gap: these are component tests. A deployment test must still install the per-user scheduled task and native-messaging registry entries on a representative Windows client before release. The generic all-user launcher remains organisation policy-dependent; the installer therefore requires a concrete user and browser extension ID.
