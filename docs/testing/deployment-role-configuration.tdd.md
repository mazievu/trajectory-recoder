# Deployment role configuration — TDD evidence

## Source and user journey

Derived from the production deployment request: a machine must have an explicit
server or client role, and a client must have an explicit destination rather
than silently sending capture data to a local endpoint.

As a deployment operator, I can validate a server or client environment file
before launch, so that server credentials cannot land on a client and clients
cannot route uploads to loopback.

## RED / GREEN record

| Stage | Command | Result |
|---|---|---|
| RED | `powershell -NoProfile -ExecutionPolicy Bypass -File tests/deployment/role-configuration.tests.ps1` | Failed because `deployment/Validate-RoleConfiguration.ps1` did not exist. |
| GREEN | `powershell -NoProfile -ExecutionPolicy Bypass -File tests/deployment/role-configuration.tests.ps1` | Passed all valid and invalid role fixtures. |
| Compose syntax | `docker compose --env-file deployment/server.env.example -f server/docker-compose.yml config` | Passed; default stack excludes the `development-only` MinIO profile. |
| TLS RED | Same role test after adding a server fixture without hostname/certificate/key inputs | Failed because the incomplete server fixture was accepted. |
| TLS GREEN | Same role test after extending the validator | Passed; a TLS proxy deployment requires hostname, certificate path, and key path. |
| Caddy syntax | `docker run ... caddy:2.10-alpine caddy adapt --config /etc/caddy/Caddyfile --adapter caddyfile` | Passed with an operator-supplied test hostname. |

## Guarantees

| # | What is guaranteed | Test | Result |
|---|---|---|---|
| 1 | A client requires `DEPLOYMENT_ROLE=client`, an HTTPS non-loopback `TRAJECTORY_SERVER_URL`, machine/user identity, bootstrap authentication, and `SPOOL_DIR`. | `client-valid.env`, missing-URL and loopback-URL fixtures | PASS |
| 2 | A server requires `DEPLOYMENT_ROLE=server` and its database, object-storage, enrolment, and dashboard credentials. | `server-valid.env` and wrong-role fixture | PASS |
| 3 | Client-only and server-only variable sets cannot be mixed by the validator. | role fixtures | PASS |
| 4 | The default compose deployment does not start bundled plaintext MinIO. | `docker compose ... config` | PASS |
| 5 | A server deployment cannot validate without public hostname and TLS certificate/key mount paths. | `server-missing-tls.env` | PASS |

## Scope and known gap

This validates the configuration contract only. Runtime rejection of an invalid
`DEPLOYMENT_ROLE` is covered by the owning client/server runtime tests. Docker
image compilation is validated separately because it compiles the server source
and depends on the current workspace toolchain. A real certificate, domain/DNS,
and reachable port 443 are required to verify an actual TLS handshake and ACME
renewal.
