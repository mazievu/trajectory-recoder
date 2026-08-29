# Server production hardening — TDD evidence

Source plan: derived during this remediation run.

## User journeys

- As an operator, I want the ingestion service to refuse startup without its
  database, object-store and credential configuration so that it cannot
  silently accept production traffic using volatile storage or a known key.
- As a registered machine, I want to upload only my own sessions so that a
  device JWT cannot read, alter or complete another device's recording.

## RED / GREEN checkpoints

| Stage | Command | Result |
| --- | --- | --- |
| RED | `cargo test -p server --test test_server_api -- --nocapture` | 4 pass, 2 fail: empty JWT secret was accepted and unauthenticated session initiation returned 200. |
| GREEN | `cargo test -p server -- --nocapture` | 16 pass, 0 fail. |
| Startup fail-closed | `cargo run -p server --bin trajectory-server` with required variables removed | Exit 1: `required environment variable DATABASE_URL is not set`. |

## Guarantees

| Guarantee | Test / evidence | Result |
| --- | --- | --- |
| Empty JWT signing secrets cannot create or verify tokens. | `jwt_rejects_an_empty_signing_secret` | PASS |
| Missing JWT is rejected and a JWT subject cannot claim another `machine_id`. | `session_initiation_requires_a_machine_jwt_and_rejects_spoofed_machine_id` | PASS |
| A different machine cannot upload to an existing session. | `session_chunks_are_only_available_to_the_machine_that_initiated_the_session` | PASS |
| Invalid enrollment credentials cannot register a machine. | `registration_rejects_an_invalid_enrollment_token` | PASS |
| Production configuration rejects short secrets and HTTP object-store endpoints. | `production_config_rejects_insecure_secrets_and_http_storage` | PASS |
| Existing hash, resumability and archive verification flows remain valid under JWT authorization. | `test_server_api`, `test_server_stress` | PASS (16 tests) |

## Known gaps

- These tests use the in-memory fixture intentionally; a PostgreSQL/S3 integration
  environment is still required to exercise the production backend itself.
- `user_id` is now required rather than defaulted, but is not bound to a separate
  user identity provider because the current device JWT contains only `machine_id`.
- `cargo clippy -p server --all-targets -- -D warnings` is blocked by the existing
  `core-types::RawEvent::new` `too_many_arguments` lint, outside this server scope.
