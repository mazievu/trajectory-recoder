# Interactive Capture Installer — TDD Evidence

Source plan: derived during this implementation run.

## User journey

As a Windows client administrator, I can install an interactive capture task
and browser native-messaging bridge for one signed-in user, then remove every
artifact created by that install.

## RED / GREEN

| Stage | Command | Result |
|---|---|---|
| RED | `powershell -NoProfile -ExecutionPolicy Bypass -File tests/deployment/interactive-capture-installer.tests.ps1` | Failed because `Install-InteractiveCaptureTask.ps1` had no `-ManifestDirectory` override, so a test could not isolate native-messaging artifacts. |
| GREEN | Same command | Passed after isolated `-NativeHostName` and `-ManifestDirectory` parameters were added, remove deleted the manifest, and the PowerShell cmdlet enum was corrected to `Interactive` (which emits XML `InteractiveToken`). |

## Guarantees

| # | What is guaranteed | Test target | Type | Result |
|---|---|---|---|---|
| 1 | The launcher creates a task whose exported Task Scheduler XML uses `InteractiveToken`. | `tests/deployment/interactive-capture-installer.tests.ps1` | Windows integration | PASS |
| 2 | The task targets the requested current user, starts the fixture agent executable, and passes the explicit client config path. | Same | Windows integration | PASS |
| 3 | The manifest points at the fixture browser-host executable and contains exactly the supplied Chrome and Edge extension origins. | Same | Windows integration | PASS |
| 4 | Chrome and Edge HKCU native-messaging registrations point at that manifest. | Same | Windows integration | PASS |
| 5 | The remove path deletes the GUID-scoped task, both registrations, and manifest. | Same | Windows integration | PASS |

## Isolation and known gaps

The check creates empty fixture executables/configuration in a unique `%TEMP%`
directory. It uses a GUID-scoped task name and host name; its `finally` block
removes only those task, registry, and temporary-file paths, including after a
failure. It does not start the capture executable, test an actual browser
extension, or capture employee activity. Those require a signed client package
and a representative deployment environment.
