# Server dashboard TDD evidence

Source plan: journeys derived during this implementation.

## User journey

As a server operator, I can sign in to the server-hosted dashboard and see each
registered recorder's identity, online/offline state, latest heartbeat, and
online duration without exposing an administrator credential to the browser
bundle.

## Task report

The static React dashboard calls the same-origin protected machines endpoint
with browser cookies only. It has no fixture records, device JWT, server token,
or `VITE_*` secret configuration. A password login creates the server-managed
session; the browser then sends its HttpOnly cookie using `credentials: include`.

RED evidence:

```text
pnpm --dir apps/desktop-ui/ui test
SyntaxError: ... does not provide an export named 'DashboardAuthenticationError'
```

This was the intended failure before cookie-authenticated dashboard behavior was
implemented.

GREEN evidence:

```text
pnpm --dir apps/desktop-ui/ui test
tests 5; pass 5; fail 0

pnpm --dir apps/desktop-ui/ui run build
vite ... built
```

| # | Guaranteed behavior | Test | Type | Result |
|---|---|---|---|---|
| 1 | Server DTOs become stable display models | `normalizes the protected machines endpoint response` | Unit | PASS |
| 2 | Online duration is human-readable | `formats online duration` | Unit | PASS |
| 3 | Machine list uses same-origin cookie auth, never a bearer/device token | `uses the cookie-authenticated same-origin endpoint` | Unit | PASS |
| 4 | Missing/expired session is handled as an authentication state | `signals an unauthenticated dashboard session` | Unit | PASS |
| 5 | The operator password is posted only to same-origin login | `submits the operator password only to the same-origin login endpoint` | Unit | PASS |

## Coverage and gaps

Node's built-in test runner is used for the browser-independent API contract;
the project has no configured coverage threshold. The React rendering and the
server's cookie issuance are verified by the production build and server-side
tests respectively. An end-to-end browser test should be added once the server
serves the static bundle and login route together.
