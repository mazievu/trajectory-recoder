# Server presence TDD evidence

Source plan: derived from the server/client deployment requirement in this task.

## User journey

As a server dashboard operator, I can see each enrolled client’s identity,
last heartbeat, current continuous online duration, and latest capture/upload
context without allowing one client’s device token to read another client.

| # | Guarantee | Test | Result | Evidence |
|---|---|---|---|---|
| 1 | A registered client’s authenticated heartbeat is shown as online with its hostname, last-seen timestamp, disk use, and active session. | `apps/server/tests/test_server_api.rs:dashboard_lists_a_registered_machine_after_its_authenticated_heartbeat` | PASS | `cargo test -p server --test test_server_api dashboard_lists_a_registered_machine_after_its_authenticated_heartbeat` |
| 2 | A stale client reconnecting after 90 seconds starts a fresh continuous online interval. | Same integration test | PASS | In-memory clock state is made stale, then the heartbeat resets `online_since_at`. |
| 3 | A device JWT cannot read the cross-machine dashboard endpoint. | Same integration test | PASS | Request with `Authorization: Bearer <device JWT>` returns `401`; only `X-Server-Token` is accepted. |

## Commands and outcomes

The initial RED command was attempted before production code changed:

```text
cargo test -p server --test test_server_api dashboard_lists_a_registered_machine_after_its_authenticated_heartbeat
```

It could not execute because concurrent Cargo processes held the shared build
directory lock. This is not presented as RED evidence. The isolated-target
GREEN run after implementation completed successfully:

```text
$env:CARGO_TARGET_DIR = "$PWD/target-server-presence"
cargo test -p server --test test_server_api dashboard_lists_a_registered_machine_after_its_authenticated_heartbeat
# 1 passed; 0 failed

cargo test -p server
# 19 passed; 0 failed
```

No Rust coverage target is configured in this workspace. Production PostgreSQL
query execution still requires an integration environment with PostgreSQL and
S3-compatible object storage; the migration is embedded and compiled, but that
external deployment path was not run in this local test.
