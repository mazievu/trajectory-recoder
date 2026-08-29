# Trajectory Recorder: Security Architecture & Threat Model

**Document Classification**: Production Security Architecture & Compliance  
**Cryptographic Standards**: Windows DPAPI, XChaCha20-Poly1305 AEAD, TLS 1.3, SHA-256  
**Compliance Targets**: Enterprise Data Privacy, Fail-Closed Redaction, Principle of Least Privilege  

---

## Table of Contents
1. [Threat Model & Security Principles](#1-threat-model--security-principles)
2. [3-Tier In-Memory Privacy Redaction Engine](#2-3-tier-in-memory-privacy-redaction-engine)
3. [Fail-Closed Masking Guarantees](#3-fail-closed-masking-guarantees)
4. [Workstation Credential Protection & DPAPI](#4-workstation-credential-protection--dpapi)
5. [Named Pipe IPC Security (SDDL)](#5-named-pipe-ipc-security-sddl)
6. [Data at Rest & In Transit Cryptography](#6-data-at-rest--in-transit-cryptography)
7. [Exclusion Policies & Windows Secure Desktops](#7-exclusion-policies--windows-secure-desktops)

---

## 1. Threat Model & Security Principles

Trajectory Recorder captures comprehensive user interaction telemetry on enterprise Windows workstations. Because telemetry streams inevitably encounter confidential corporate information, user passwords, and customer PII, the system is designed under a strict **Zero-Trust, Fail-Closed Security Model**.

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                   THREAT MODEL MATRIX                                   │
├───────────────────────────┬───────────────────────────────┬─────────────────────────────┤
│ Threat Vector             │ Potential Impact              │ Architectural Mitigation    │
├───────────────────────────┼───────────────────────────────┼─────────────────────────────┤
│ Credential Leakage in UI  │ Password or API token logged  │ 3-tier in-memory redaction; │
│                           │ to disk or database           │ UIA password suppression    │
├───────────────────────────┼───────────────────────────────┼─────────────────────────────┤
│ Local Disk Compromise     │ Attacker reads spool folder   │ XChaCha20-Poly1305 AEAD     │
│ (Unprivileged user/malware│ contents on workstation       │ encryption; DPAPI keys      │
├───────────────────────────┼───────────────────────────────┼─────────────────────────────┤
│ IPC Pipe Tampering /      │ Local process injects fake    │ Named Pipe SDDL restricting │
│ Injection Attacks         │ commands or intercepts data   │ access to SYSTEM/Admins/IU  │
├───────────────────────────┼───────────────────────────────┼─────────────────────────────┤
│ Network Eavesdropping /   │ Man-in-the-Middle intercepting│ TLS 1.3 HTTPS; client JWT   │
│ Man-in-the-Middle (MitM)  │ uploaded session chunks       │ authentication; AEAD chunks │
├───────────────────────────┼───────────────────────────────┼─────────────────────────────┤
│ Chunk Tampering / Bit Flip│ Corrupted or maliciously      │ SHA-256 checksum headers;   │
│ in Cloud Storage          │ modified trajectory data      │ Poly1305 AEAD MAC tags      │
└───────────────────────────┴───────────────────────────────┴─────────────────────────────┘
```

---

## 2. 3-Tier In-Memory Privacy Redaction Engine

The `crates/privacy` engine processes every intercepted string, UI element attribute, clipboard content, and canonical action parameter **in-memory before it reaches disk, memory queues, or IPC streams**.

```
Raw Telemetry (Typed Text, Clipboard, UIA Element, Window Title)
                              │
                              ▼
        ┌───────────────────────────────────────────┐
        │ TIER 1: Password Box & Field Exclusion    │
        │ • UIA IsPassword flag check               │
        │ • ControlType == Password / SecureEdit    │
        └─────────────────────┬─────────────────────┘
                              │ Redacted to [PASSWORD_REDACTED]
                              ▼
        ┌───────────────────────────────────────────┐
        │ TIER 2: Deterministic Regex & Algorithms  │
        │ • Social Security Numbers (SSN)           │
        │ • Credit Cards (Regex + Luhn Validation)  │
        │ • Cloud API Keys (AWS, GitHub, Stripe)    │
        │ • JSON Web Tokens (Bearer JWT)            │
        │ • Basic Auth URLs (https://user:pass@)    │
        └─────────────────────┬─────────────────────┘
                              │ Redacted to [TOKEN_TYPE_REDACTED]
                              ▼
        ┌───────────────────────────────────────────┐
        │ TIER 3: Shannon Entropy Anomaly Filter    │
        │ • High-entropy secret token detection     │
        │ • $H(X) > 4.5\text{ bits/char}$ (len ≥ 16)│
        └─────────────────────┬─────────────────────┘
                              │ Redacted to [HIGH_ENTROPY_REDACTED]
                              ▼
               Cleaned, Sanitized Output Stream
```

### 2.1 Tier 1: UI Automation Password Box Exclusion
- **Mechanism**: The UI Automation tree walker inspects the `IsPassword` COM property on all target controls.
- **Behavior**: If `IsPassword == true`, typed characters and element values are suppressed at the source. The value is unconditionally overwritten with `"[PASSWORD_REDACTED]"`.

### 2.2 Tier 2: Deterministic Regex Rules & Algorithmic Validation
- **Social Security Numbers (SSN)**:
  `\b\d{3}-\d{2}-\d{4}\b` → `[SSN_REDACTED]`
- **Credit Card Numbers (Luhn Algorithm)**:
  Candidates matching `\b(?:\d[ -]*?){13,19}\b` are verified using the Luhn modulo-10 algorithm. Valid credit card numbers are masked to `[CREDIT_CARD_REDACTED]`, while ordinary numerical sequences (such as order IDs or tracking numbers) are preserved.
- **API Keys & Cloud Tokens**:
  - AWS Access Keys: `AKIA[0-9A-Z]{16}` → `[API_KEY_REDACTED]`
  - GitHub Tokens: `ghp_[0-9a-zA-Z]{36}` → `[API_KEY_REDACTED]`
  - Stripe Secret Keys: `sk_live_[0-9a-zA-Z]{24}` → `[API_KEY_REDACTED]`
  - Bearer JWT Tokens: `Bearer\s+[A-Za-z0-9\-_=]+\.[A-Za-z0-9\-_=]+\.?[A-Za-z0-9\-_=]*` → `Bearer [API_KEY_REDACTED]`
- **URL Embedded Passwords**:
  `https?://([^:]+):([^@]+)@` → `https://$1:[PASSWORD_REDACTED]@`

### 2.3 Tier 3: Shannon Entropy Filter
Random cryptographic secrets, base64 strings, and hex hashes that do not match known regex patterns are detected via Shannon Entropy:

$$H(X) = -\sum_{i=1}^{n} P(x_i) \log_2 P(x_i)$$

- Strings with length $\ge 16$ characters and entropy $H > 4.5\text{ bits per character}$ are classified as high-entropy secrets and replaced with `"[HIGH_ENTROPY_REDACTED]"`.

---

## 3. Fail-Closed Masking Guarantees

If any stage of the privacy engine encounters an internal error, memory allocation fault, or parser ambiguity:
1. The engine **fails closed**: the target field is overwritten with `"[REDACTED]"` or `"[UNOBSERVED_TEXT]"`.
2. Raw keystroke buffers are zeroed immediately in memory using `zeroize`.
3. Under no circumstances is unredacted text written to `events.raw.ndjson`, `events.normalized.ndjson`, or `session.db`.

---

## 4. Workstation Credential Protection & DPAPI

Workstation-side identity secrets and master encryption keys are secured using the **Windows Data Protection API (DPAPI)**:

```rust
// CryptProtectData with CRYPTPROTECT_LOCAL_MACHINE | CRYPTPROTECT_UI_FORBIDDEN
let protected = Dpapi::protect_machine_secret(&master_key_bytes, None)?;
```

### Key Management Policies
- **No Hardcoded Secrets**: Source code contains zero hardcoded cryptographic keys, database credentials, or default JWT secrets.
- **Machine Identity Token**: On enrollment, `trajectory-supervisor` receives a signed JWT from the server, encrypts it via DPAPI with `CRYPTPROTECT_LOCAL_MACHINE`, and persists it to `C:\ProgramData\TrajectoryRecorder\device_identity.enc`.
- **Session 0 to Session 1 Access**: Machine-level DPAPI protection ensures that both the Session 0 supervisor and interactive session agents can decrypt the shared machine identity without exposing plaintext keys to unprivileged user files.

---

## 5. Named Pipe IPC Security (SDDL)

All Windows Named Pipes (`\\.\pipe\trajectory-agent-ipc`, `\\.\pipe\trajectory-browser-host`) are initialized with strict Security Descriptor Definition Language (SDDL) strings:

```sddl
D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;IU)
```

### Access Control Matrix
- **`SY` (NT AUTHORITY\SYSTEM)**: Full Generic All (`GA`) permissions.
- **`BA` (Builtin Administrators)**: Full Generic All (`GA`) permissions.
- **`IU` (Interactive Users)**: Full Generic All (`GA`) permissions for local user agents.
- **`NU` (Network Users) / Anonymous**: Explicitly denied. Named Pipes are configured for local inter-process communication only (`FILE_PIPE_LOCAL`).

---

## 6. Data at Rest & In Transit Cryptography

### 6.1 Data at Rest (Local Spool & Chunks)
Finalized session packages are compressed via Zstandard and encrypted with **XChaCha20-Poly1305 AEAD**:
- **Key Size**: 256 bits (32 bytes).
- **Nonce Size**: 192 bits (24 bytes) generated with hardware RNG (`OsRng`).
- **Authentication Tag**: 128 bits (16 bytes) Poly1305 MAC.
- **Associated Authenticated Data (AAD)**: Slices are bound to `{session_id}_chunk_{chunk_index}` to prevent chunk substitution attacks.

### 6.2 Data in Transit (HTTPS REST)
- All client-to-server traffic traverses **TLS 1.3** using secure cipher suites (`TLS_AES_256_GCM_SHA384`, `TLS_CHACHA20_POLY1305_SHA256`).
- Upload requests carry individual chunk integrity hashes in the `X-Chunk-SHA256` HTTP header.

### 6.3 Cloud Object Storage
- S3 / MinIO buckets enforce Server-Side Encryption (SSE-S3 or SSE-KMS) alongside client-side XChaCha20 encryption (double envelope encryption).

---

## 7. Exclusion Policies & Windows Secure Desktops

### 7.1 Excluded Applications & Windows
Administrators can configure system-wide exclusion rules in `config.toml`:
```toml
[privacy]
excluded_apps = ["KeePass.exe", "1Password.exe", "Bitwarden.exe", "cmdkey.exe"]
excluded_window_titles = ["Windows Security", "User Account Control", "Credential Manager"]
excluded_domains = ["bank.com", "vault.internal"]
```
When an excluded process or window title attains foreground focus:
1. Input capture and screenshot hooks are immediately suspended.
2. The session logs an empty `WINDOW_SWITCH` event with process name `"EXCLUDED_APPLICATION"`.
3. Capture resumes only after the user switches focus to an unexcluded window.

### 7.2 Windows Secure Desktop (UAC & Winlogon)
- Windows enforces kernel-level isolation when the User Account Control (UAC) elevation prompt or `Winlogon` (Ctrl+Alt+Del) desktop is active.
- `WH_MOUSE_LL` and `WH_KEYBOARD_LL` hooks installed in interactive sessions cannot receive events from the Secure Desktop.
- Trajectory Recorder natively respects this OS boundary, ensuring administrative credentials and Windows login passwords are never captured.
