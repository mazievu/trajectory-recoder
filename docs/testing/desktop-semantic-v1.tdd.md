# Desktop Semantic Capture v1 — TDD Evidence

## Source and journeys

This work was derived from the interactive desktop-capture audit, not from a
plan file.

- As a recorder user, I want a real Win32 press-and-release gesture to appear
  as a semantic Click, so that a trajectory describes an application action
  rather than only mouse coordinates.
- As a recorder user, I want UI Automation metadata only for meaningful
  interactions, so that pointer movement does not exhaust the UIA worker while
  typing and clicks still retain their target.

## RED / GREEN record

| Behaviour | RED evidence | GREEN evidence | Guarantee |
|---|---|---|---|
| A left `MOUSE_DOWN` followed by a matching `MOUSE_UP` becomes a Click | `cargo test -p correlator mouse_down_then_up_emits_click_with_release_target` failed: expected 1 action, got 0 | Same command passed after the correlator change | The click uses the release target metadata, while a drag remains a DragDrop action. |
| UIA is not queried for raw mouse movement | `cargo test -p capture-agent uia_lookup_ignores_mouse_moves_but_keeps_semantic_targets` failed at compile time because the policy did not exist | Same command passed after adding the policy | Mouse move and key-up do not request UIA; mouse release requests point metadata and key-down requests focused metadata. |

## Validation

| # | What is guaranteed | Test / command | Type | Result |
|---|---|---|---|---|
| 1 | Win32 down/up produces Click with UIA release target | `cargo test -p correlator` | Unit | PASS — 4 tests |
| 2 | Drag gesture is not misclassified as a click | `cargo test -p correlator` | Unit | PASS — existing drag-drop test |
| 3 | Semantic UIA lookup policy excludes mouse movement | `cargo test -p capture-agent` | Unit | PASS — 1 test |
| 4 | Workspace targets compile after the change | `cargo check --workspace --all-targets` | Build | PASS, with pre-existing warnings |

## Coverage and known gaps

`cargo llvm-cov` is not installed in this environment, so no coverage
percentage was measured. The new pure decision branches are unit-tested, but
an interactive Windows UIA run remains required to verify element metadata
from a native application. The implementation samples focused UIA metadata on
foreground and key-down events; it does not yet subscribe to the full Windows
UI Automation focus/property-change event stream.

## Commit evidence

- `7842431` — RED test for physical Win32 down/up.
- `022eb58` — GREEN correlator fix.
- `f05a2fb` — RED test for semantic UIA policy.
- `95a24de` — GREEN capture-agent policy fix.
