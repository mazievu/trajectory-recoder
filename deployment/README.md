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

The server compose stack exposes only the HTTPS proxy on port 443. PostgreSQL
and the ingestion server are private Compose-network services; `/dashboard` and
the ingestion API are both proxied by the same public hostname.

Before starting it, obtain a certificate for `PUBLIC_HOSTNAME` and set
`TLS_CERT_PATH` and `TLS_KEY_PATH` to operator-managed PEM files. The paths are
read-only mounts; no private key is copied into the image or repository.

```powershell
docker compose --env-file deployment/server.env -f server/docker-compose.yml up -d --build
```

This starts the ingestion server and PostgreSQL. It requires an externally
managed **HTTPS** S3-compatible object store in `S3_ENDPOINT`; it does not use
the bundled MinIO container.

### Certificate options

The shipped compose path uses an operator-provided certificate/key. This works
with internal PKI, a public CA, or a certificate acquired with ACME outside the
stack. For public ACME, run the ACME client on the host or a separately reviewed
edge proxy, renew the files in place, then reload the proxy. This stack does not
expose port 80 or silently create an ACME account.

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
  not the server's private Docker address or `BIND_ADDR`; it must match
  `https://PUBLIC_HOSTNAME`.

For a single-machine development demo only, use a separate development override;
do not weaken this deployment contract.

## Interactive capture launcher (Windows client)

`trajectory-supervisor` and `trajectory-uploader` are Session 0 processes. The
capture agent must not be started there: Windows input hooks and UI Automation
would observe Session 0, not the signed-in employee's desktop.

After placing the signed client binaries and the validated `client.env` on a
Windows client, install the per-user launcher while signed in as that user:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File deployment/Install-InteractiveCaptureTask.ps1 `
  -ConfigPath C:\ProgramData\TrajectoryRecorder\client.env `
  -InstallDirectory 'C:\Program Files\TrajectoryRecorder' `
  -UserId 'CONTOSO\operator' `
  -ChromeExtensionId '<32-character-production-extension-id>'
```

The script creates an `InteractiveToken` logon task for `trajectory-agent.exe`.
It also registers an HKCU native-messaging manifest restricted to the supplied
extension ID. Chrome/Edge launches `trajectory-browser-host.exe` on demand via
native messaging; it is intentionally **not** a scheduled Session 0 process.

To remove that user's launcher, repeat the command with the same task name and
extension IDs, adding `-Remove`. Removal unregisters the task, deletes the
Chrome/Edge HKCU registrations, and deletes its native-messaging manifest.

On a Windows validation machine, run the isolated installer integration check:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tests/deployment/interactive-capture-installer.tests.ps1
```

It creates only a GUID-named temporary Scheduled Task, per-user native-messaging
keys under a GUID-named host, and files under `%TEMP%`. It asserts the task XML
uses `InteractiveToken`, verifies the manifest's `allowed_origins`, then calls
the remove path and asserts every temporary task, registry key, and manifest is
gone.

This is a per-user installation boundary. A generic all-user launcher requires
an organisation-specific account-provisioning policy and an extension ID; the
installer therefore requires both explicitly instead of guessing either.

## Validation boundary

The included checks validate role inputs, Compose interpolation, Caddyfile syntax,
and that only the proxy publishes an application port. They cannot prove a
certificate matches `PUBLIC_HOSTNAME`, that a certificate chain is trusted, DNS
points at this host, port 443 is reachable, or ACME renewal works. Those require
a real certificate, a routable domain, and an end-to-end deployment test.
