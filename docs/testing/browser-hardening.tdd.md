# Browser Hardening TDD Evidence

## Scope

Journeys derived from the production privacy and browser-capture requirements:

1. A form edit must never become plaintext browser telemetry or persisted target metadata.
2. A browser-host restart must not reuse a real global event ID; only capture-agent owns durable allocation.
3. The extension must compile before its unpacked `dist/` output is loaded by Chrome or Edge.

## RED

`cargo test -p browser-events` failed before implementation because
`BrowserDomEvent::to_unassigned_raw_event` did not exist. This test was
committed as `fd9d1e5`.

## GREEN

| Guarantee | Evidence | Result |
| --- | --- | --- |
| A received form value maps to `[UNOBSERVED_TEXT]`, never its plaintext | `cargo test -p browser-events` | 3 passed |
| Browser-host source event IDs increase and use only `GlobalEventId(0)` in transit | `cargo test -p browser-host` | 1 passed |
| Tier-1 DOM metadata test does not expect a plaintext form value | `cargo test -p tier1-feature test_f26_browser_dom_events_and_xpath` | 1 passed |
| Extension TypeScript and manifest are valid | `npm run check`, `npm run build`, JSON parse | passed |

## Known integration gate

`GlobalEventId(0)` is intentionally an unassigned sentinel. Capture-agent must
replace it using `GlobalEventIdAllocator` before publication or persistence.
The Chromium native-host registration also needs a deployed, fixed extension ID;
the repository's wildcard origin manifest is not a valid production registration.
