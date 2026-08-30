# Deployment roles

Each installed machine has exactly one deployment role. The role is deliberately
explicit so capture clients cannot silently upload to themselves or inherit server
credentials.

| Role | Runs | Does not run | Required configuration |
|---|---|---|---|
| `server` | `trajectory-server`, PostgreSQL, object storage, and the server dashboard | capture agent, browser host, uploader, or client tray | `deployment/server.env` |
| `client` | supervisor, capture agent, browser host, and uploader as background processes | server, PostgreSQL, object storage, and server dashboard | `deployment/client.env` |

Copy the matching example file; never copy one machine's actual `.env` file to
another machine. The enrolment token is per client; after first enrolment the
uploader stores its device credential with Windows DPAPI.

```powershell
Copy-Item deployment/server.env.example deployment/server.env
powershell -NoProfile -ExecutionPolicy Bypass -File deployment/Validate-RoleConfiguration.ps1 `
  -ConfigPath deployment/server.env -ExpectedRole server

Copy-Item deployment/client.env.example deployment/client.env
powershell -NoProfile -ExecutionPolicy Bypass -File deployment/Validate-RoleConfiguration.ps1 `
  -ConfigPath deployment/client.env -ExpectedRole client
```

The server compose stack consumes the server file directly:

```powershell
docker compose --env-file deployment/server.env -f server/docker-compose.yml up -d --build
```

This starts the ingestion server and PostgreSQL. It requires an externally
managed **HTTPS** S3-compatible object store in `S3_ENDPOINT`; it does not use
the bundled MinIO container.

`server/docker-compose.yml` contains a MinIO profile named `development-only`
solely for local dependency experiments. It is excluded from the default stack
and must never be used as a plaintext production object-store endpoint. A local
MinIO deployment needs TLS termination before it can be used with the server.

The client validator rejects a missing endpoint, `http`, `localhost`, and loopback
addresses. The server validator rejects client-only variables and a loopback bind
address. Validation prints variable names only; it never prints secret values.

## Runtime contract

The launcher/runtime must reject a mismatch between its binary and
`DEPLOYMENT_ROLE`. In particular:

- a client must require `DEPLOYMENT_ROLE=client`, `TRAJECTORY_SERVER_URL`,
  `TRAJECTORY_MACHINE_ID`, `TRAJECTORY_USER_ID`, and bootstrap authentication;
  it must not default an omitted endpoint to `127.0.0.1`;
- a server must require `DEPLOYMENT_ROLE=server` and the server storage/auth
  variables; it must not start the capture stack;
- the client endpoint is the reverse-proxy HTTPS URL visible to client machines,
  not the server's private Docker address or `BIND_ADDR`.

For a single-machine development demo only, use a separate development override;
do not weaken this deployment contract.
