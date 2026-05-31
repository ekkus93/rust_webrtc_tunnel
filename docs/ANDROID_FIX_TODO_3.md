# Android WebRTC Tunnel Fix TODO 3

## 1. Goal

Finish the remaining Android app hardening after `ANDROID_FIX_TODO_2`.

This pass is not a redesign. It is a correctness, lifecycle, security, and validation-honesty pass.

Highest-priority outcomes:

```text
Checklist claims are honest.
ForegroundService does not block main thread during startup.
Network policy UI matches service enforcement.
Logs are redacted before display/export.
Generated TOML is safely serialized.
Android offer ↔ desktop answer E2E validation is documented.
```

## 2. Rules

- [ ] Keep Android work on the Android feature branch unless the user explicitly says otherwise.
- [ ] Do not merge to `master` until validation passes.
- [ ] Do not change MQTT signaling wire format.
- [ ] Do not change tunnel frame format.
- [ ] Do not change desktop Rust protocol semantics.
- [ ] Do not add TURN.
- [ ] Do not add VPN/TUN mode.
- [ ] Do not add arbitrary remote host/port selection from Android offer side.
- [ ] Do not allow cellular/metered data unless explicitly enabled by the user.
- [ ] Do not store private identity plaintext at rest.
- [ ] Do not log private keys, MQTT passwords, SDP, ICE candidates, decrypted payloads, or forwarded data.
- [ ] Bind local forwards to `127.0.0.1` by default.
- [ ] Do not check off any acceptance item unless implementation and validation are complete.
- [ ] If a validation command cannot be run, document why and leave the related acceptance item unchecked.

---

# Phase 0 — Reset checklist honesty

## 0.1 Audit over-checked items

Audit:

```text
ANDROID_WEBRTC_TUNNEL_TODO.md
ANDROID_FIX_TODO1.md
ANDROID_FIX_TODO_2.md
ANDROID_FIX_TODO_2(1).md, if present
docs/memory.md
docs/ANDROID_VALIDATION.md
```

Uncheck or annotate any item not proven by test or documented validation.

Pay special attention to:

- [ ] Android offer connects to desktop Rust answer.
- [ ] Android browser reaches remote service through `127.0.0.1:<port>`.
- [ ] end-to-end validation is documented.
- [ ] setup wizard is truly complete.
- [ ] forwards screen has all requested actions.
- [ ] network policy UI matches service behavior.
- [ ] logs are redacted before display.
- [ ] foreground-service lifecycle is fully hardened.
- [ ] JNI/FFI destroy/dispose is fully safe.
- [ ] final acceptance checklist is complete.

## 0.2 Define evidence standard

For every checked item, add one of:

- [ ] automated test name;
- [ ] validation command and result;
- [ ] manual E2E validation record;
- [ ] documented reason if intentionally not implemented.

## 0.3 Update validation docs

Update:

```text
docs/ANDROID_VALIDATION.md
```

Required fields:

- [ ] date;
- [ ] git commit;
- [ ] environment;
- [ ] command list;
- [ ] pass/fail result for each command;
- [ ] not-run reason for unavailable commands;
- [ ] unresolved failures.

## 0.4 Acceptance

- [ ] No checklist item is checked without evidence.
- [ ] E2E items are unchecked unless real Android↔desktop validation is documented.
- [ ] Validation docs distinguish PASS, FAIL, and NOT RUN.

---

# Phase 1 — Document or rerun Android↔desktop E2E validation

## 1.1 Prepare desktop answer

Document:

- [ ] desktop OS/environment;
- [ ] git commit;
- [ ] exact desktop answer command;
- [ ] desktop config path;
- [ ] desktop public identity;
- [ ] MQTT broker host/port/TLS summary, secrets redacted;
- [ ] remote service being forwarded;
- [ ] remote forward ID.

Example:

```bash
cargo run --bin p2p-answer -- --config /path/to/answer-config.toml
```

## 1.2 Prepare Android offer using UI only

Document:

- [ ] Android device/emulator model;
- [ ] Android API level;
- [ ] app build variant/APK;
- [ ] network type;
- [ ] identity import/generation;
- [ ] remote public identity import;
- [ ] MQTT broker settings, secrets redacted;
- [ ] configured forward;
- [ ] network policy setting.

## 1.3 Run browser test

On Android, open:

```text
http://127.0.0.1:<local_port>
```

Record:

- [ ] URL;
- [ ] expected remote service;
- [ ] response status or visible result;
- [ ] response body summary;
- [ ] redacted Android logs;
- [ ] redacted desktop logs;
- [ ] pass/fail result.

## 1.4 If E2E cannot be run

Document:

- [ ] `NOT RUN`;
- [ ] exact reason;
- [ ] missing dependency/environment;
- [ ] exact steps to run later.

Leave these unchecked:

- [ ] Android offer connects to desktop Rust answer.
- [ ] Android browser reaches remote service through localhost.
- [ ] E2E validation complete.

## 1.5 Acceptance

- [ ] `docs/ANDROID_VALIDATION.md` contains real E2E evidence or explicit NOT RUN reason.
- [ ] No E2E acceptance item is checked without evidence.

---

# Phase 2 — Move ForegroundService startup work off main thread

## 2.1 Audit current service startup path

Inspect:

```text
TunnelForegroundService
TunnelRepository
ConfigRepository
IdentityRepository
NetworkPolicyManager
RustTunnelBridge
```

Document every operation currently performed synchronously in `onStartCommand()` or directly called from it:

- [ ] DataStore reads;
- [ ] file I/O;
- [ ] identity decrypt;
- [ ] Rust config validation;
- [ ] Rust runtime startup;
- [ ] network policy checks;
- [ ] notification updates.

## 2.2 Required service threading model

Implement:

- [ ] `onCreate()` creates notification channel and calls `startForeground()` promptly.
- [ ] `onStartCommand()` parses action only.
- [ ] `onStartCommand()` launches service work on `serviceScope`.
- [ ] startup I/O and native calls run on `Dispatchers.IO` or equivalent.
- [ ] no `runBlocking` remains on main service path.
- [ ] service state updates are synchronized and race-safe.
- [ ] duplicate START actions do not start duplicate native runtimes.
- [ ] STOP action can interrupt pending startup safely.

## 2.3 Startup flow

Implement asynchronous flow:

```text
startForeground(paused/starting notification)
launch serviceScope {
  load preferences
  check network policy
  if blocked: update paused state and return
  read/decrypt identity
  validate config with identity
  start native runtime
  update running notification/state
}
```

Tasks:

- [ ] make startup idempotent;
- [ ] handle cancellation;
- [ ] handle config validation error;
- [ ] handle identity missing/error;
- [ ] handle network blocked;
- [ ] handle native startup failure;
- [ ] update UI/repository state for each failure.

## 2.4 Stop flow

STOP must:

- [ ] cancel pending startup job;
- [ ] stop native runtime idempotently;
- [ ] unregister network callbacks;
- [ ] update state to stopped;
- [ ] stop foreground notification;
- [ ] stop self;
- [ ] avoid leaking coroutine jobs.

## 2.5 Tests

Add/update tests:

- [ ] service start posts notification promptly;
- [ ] startup work runs off main thread;
- [ ] no `runBlocking` in service startup path;
- [ ] duplicate START does not double-start runtime;
- [ ] STOP during pending startup is safe;
- [ ] native startup failure produces actionable error state;
- [ ] null intent does not crash;
- [ ] start-stop-start works;
- [ ] no `ForegroundServiceDidNotStartInTimeException`.

## 2.6 Acceptance

- [ ] ForegroundService performs no blocking startup work on main thread.
- [ ] Service remains compliant with Android foreground-service timing.
- [ ] Start/stop lifecycle is reliable.

---

# Phase 3 — Fix network policy consistency

## 3.1 Define single policy function

Create one shared policy calculation used by both service and UI.

Required inputs:

- [ ] network type;
- [ ] metered status;
- [ ] `allowMetered`;
- [ ] `resumeOnUnmetered`;
- [ ] optional `pauseOnMetered`, if retained.

Required outputs:

- [ ] `networkType`;
- [ ] `isMetered`;
- [ ] `allowedByDefault`;
- [ ] `allowedByUserPolicy`;
- [ ] `blockedReason`;
- [ ] `tunnelAllowed`.

Required policy:

```text
Unmetered Wi-Fi: allowed
Metered Wi-Fi: allowed only if allowMetered = true
Cellular: allowed only if allowMetered = true
No network: blocked
Unknown: blocked always
```

## 3.2 Resolve `pauseOnMetered`

Choose one:

### Option A — remove it

- [ ] remove `pauseOnMetered` from preferences;
- [ ] remove from UI;
- [ ] remove from tests/docs;
- [ ] migrate old preference safely.

### Option B — implement it

Define semantics:

```text
pauseOnMetered = true means an already-running tunnel pauses on metered/cellular transition,
even if allowMetered is true for manual starts.
```

Tasks if Option B:

- [ ] service honors `pauseOnMetered`;
- [ ] UI explains behavior;
- [ ] tests cover interaction with `allowMetered`.

## 3.3 Update Network Policy UI

UI must show:

- [ ] current network type;
- [ ] metered/unmetered state;
- [ ] default policy result;
- [ ] user policy result;
- [ ] blocked reason;
- [ ] whether tunnel can start now;
- [ ] current `allowMetered`;
- [ ] current `resumeOnUnmetered`;
- [ ] warning before enabling metered/cellular.

## 3.4 Update service

Service must:

- [ ] use same policy function as UI;
- [ ] block startup before native runtime on disallowed network;
- [ ] pause/stop running runtime on disallowed transition;
- [ ] resume only when allowed and configured;
- [ ] keep Unknown blocked always;
- [ ] surface blocked reason to Home/logs/notification.

## 3.5 Tests

Add tests:

- [ ] unmetered Wi-Fi allowed by default;
- [ ] metered Wi-Fi blocked by default;
- [ ] cellular blocked by default;
- [ ] metered Wi-Fi allowed when `allowMetered = true`;
- [ ] cellular allowed when `allowMetered = true`;
- [ ] unknown blocked even when `allowMetered = true`;
- [ ] no network blocked;
- [ ] UI policy matches service policy;
- [ ] warning required before enabling metered/cellular;
- [ ] pause/resume behavior matches selected `pauseOnMetered` policy.

## 3.6 Acceptance

- [ ] Network Policy screen and service produce the same allowed/blocked answer.
- [ ] Unknown network always fails safe.
- [ ] Cellular/metered cannot be used by default.

---

# Phase 4 — Redact logs before display/export

## 4.1 Create shared redaction layer

Implement a single redaction component used by:

- [ ] Logs screen display;
- [ ] copy logs;
- [ ] diagnostics export;
- [ ] native log ingestion if appropriate;
- [ ] status/error display where useful.

## 4.2 Redaction targets

Add patterns/tests for:

- [ ] `sign.private`;
- [ ] `kex.private`;
- [ ] private identity TOML blocks;
- [ ] private key PEM blocks;
- [ ] MQTT password fields;
- [ ] password file contents if ever read;
- [ ] bearer tokens;
- [ ] API keys;
- [ ] URL credentials, e.g. `mqtts://user:pass@example.com`;
- [ ] SDP blobs;
- [ ] ICE candidates;
- [ ] decrypted payload markers;
- [ ] forwarded data markers;
- [ ] temp identity paths;
- [ ] MQTT username if considered sensitive;
- [ ] any native last-error details that may include secrets.

## 4.3 LogsViewModel

Update:

- [ ] native logs are redacted before entering UI state;
- [ ] filter works on redacted logs;
- [ ] copied logs are redacted;
- [ ] exported diagnostics use same redacted logs;
- [ ] malformed native log JSON surfaces visible redacted error.

## 4.4 DiagnosticsRepository

Update:

- [ ] status JSON is redacted before export;
- [ ] config is redacted;
- [ ] logs are redacted;
- [ ] network state included safely;
- [ ] app/native version included safely;
- [ ] raw secrets cannot appear in diagnostics.

## 4.5 Tests

Add realistic multiline tests:

- [ ] private identity TOML redacted;
- [ ] MQTT password redacted;
- [ ] bearer token redacted;
- [ ] URL credentials redacted;
- [ ] SDP multiline blob redacted;
- [ ] ICE candidate redacted;
- [ ] forwarded data marker redacted;
- [ ] native log display redacted;
- [ ] copy logs redacted;
- [ ] diagnostics export redacted;
- [ ] status JSON last error redacted.

## 4.6 Acceptance

- [ ] Logs are safe before display.
- [ ] Copied logs are safe.
- [ ] Diagnostics are safe to share.
- [ ] Redaction tests cover realistic secret formats.

---

# Phase 5 — Make generated TOML safe

## 5.1 Audit raw interpolation

Inspect every TOML-producing path:

```text
ConfigRepository.defaultConfigTemplate()
ConfigRepository.renderOfferConfig()
ConfigRepository.redactConfig()
tests/fakes that produce config TOML
```

Identify all interpolated fields:

- [ ] broker host;
- [ ] broker port;
- [ ] username;
- [ ] password path;
- [ ] topic prefix;
- [ ] local peer ID;
- [ ] remote peer ID;
- [ ] authorized keys path;
- [ ] identity path;
- [ ] state/runtime paths;
- [ ] forward ID;
- [ ] forward bind host;
- [ ] forward bind port;
- [ ] CA path, if present.

## 5.2 Implement TOML-safe serialization

Preferred:

- [ ] introduce structured config object;
- [ ] serialize with TOML library.

Acceptable:

- [ ] implement `tomlString(value: String): String`;
- [ ] escape backslash;
- [ ] escape quote;
- [ ] escape newline;
- [ ] escape carriage return;
- [ ] escape tab;
- [ ] cover Unicode safely;
- [ ] use helper for every TOML string value.

## 5.3 Tests

Add tests:

- [ ] quotes in broker host do not inject TOML;
- [ ] newline in topic prefix is escaped/rejected safely;
- [ ] quote in remote peer ID cannot inject config;
- [ ] quote in forward ID cannot inject config;
- [ ] backslash in path is preserved/escaped;
- [ ] rendered config validates or fails with actionable validation error;
- [ ] malicious-looking input cannot add extra `[[forwards]]`.

## 5.4 Acceptance

- [ ] No raw user/imported value is inserted into TOML without escaping/serialization.
- [ ] Generated TOML is robust against malformed input and injection.

---

# Phase 6 — Validate duplicate runtime forward IDs

## 6.1 Add validation

In Android forwards validation:

- [ ] reject duplicate enabled local ports;
- [ ] reject duplicate enabled `remoteForwardId`;
- [ ] reject blank `remoteForwardId`;
- [ ] reject invalid local port;
- [ ] reject arbitrary remote host/port fields;
- [ ] keep local host default `127.0.0.1`.

## 6.2 Error messages

Use actionable errors:

```text
Duplicate local port: 8080
Duplicate remote forward ID: llama
Remote forward ID is required
```

## 6.3 Tests

Add tests:

- [ ] duplicate enabled `remoteForwardId` rejected;
- [ ] duplicate disabled `remoteForwardId` behavior is documented and tested;
- [ ] duplicate local port still rejected;
- [ ] blank remote forward ID rejected;
- [ ] valid unique forwards accepted;
- [ ] generated Rust config uses unique forward IDs.

## 6.4 Acceptance

- [ ] Android cannot render duplicate Rust forward IDs.
- [ ] Forward validation errors are clear.

---

# Phase 7 — Finish Setup Wizard honestly

## 7.1 Choose Mode step

- [ ] Offer mode enabled and default.
- [ ] Answer mode disabled or clearly marked incomplete/advanced if not supported.
- [ ] User cannot accidentally configure unsupported mode.

## 7.2 Identity step

Required:

- [ ] Generate Identity action, if Rust helper available.
- [ ] Import Private Identity action.
- [ ] Validate private identity.
- [ ] Store private identity encrypted.
- [ ] Show public identity.
- [ ] Copy public identity.
- [ ] Share/export public identity, or uncheck/document if not implemented.
- [ ] Clear setup-required error if identity is missing.

## 7.3 MQTT Broker step

Required fields:

- [ ] broker host;
- [ ] port;
- [ ] TLS enabled/disabled if supported;
- [ ] TLS default-root behavior documented in UI or help text;
- [ ] username optional;
- [ ] password optional/path handled safely;
- [ ] topic prefix if supported.

Validation:

- [ ] host required;
- [ ] port valid;
- [ ] TLS settings valid;
- [ ] secrets not logged.

## 7.4 Remote Peer step

Required:

- [ ] remote peer ID;
- [ ] remote public identity text;
- [ ] paste action;
- [ ] import file action, or uncheck/document if not implemented;
- [ ] validate public identity;
- [ ] write `authorized_keys`;
- [ ] avoid duplicate entries;
- [ ] show validation errors.

## 7.5 Forwards step

Required:

- [ ] add forward;
- [ ] edit forward;
- [ ] delete forward;
- [ ] enable/disable forward;
- [ ] local host defaults to `127.0.0.1`;
- [ ] local port;
- [ ] remote forward ID;
- [ ] no arbitrary remote host/port;
- [ ] duplicate validation, including `remoteForwardId`.

## 7.6 Network Policy step

Required:

- [ ] current network type;
- [ ] metered/unmetered state;
- [ ] allowed/blocked status;
- [ ] blocked reason;
- [ ] allow metered toggle;
- [ ] metered/cellular warning before enabling;
- [ ] resume-on-unmetered option;
- [ ] Unknown blocked explanation.

## 7.7 Review step

Show:

- [ ] mode;
- [ ] local public identity;
- [ ] remote peer;
- [ ] broker;
- [ ] network policy;
- [ ] forwards.

Actions:

- [ ] Back;
- [ ] Save;
- [ ] Start Tunnel.

Start Tunnel must:

- [ ] save config atomically;
- [ ] validate config with identity;
- [ ] check identity presence;
- [ ] check network policy;
- [ ] start ForegroundService if allowed;
- [ ] show actionable blocked/error message.

## 7.8 Tests

Add/update tests:

- [ ] cannot proceed from invalid step;
- [ ] wizard creates valid config;
- [ ] wizard writes authorized_keys;
- [ ] wizard stores identity encrypted;
- [ ] wizard rejects duplicate local ports;
- [ ] wizard rejects duplicate remote forward IDs;
- [ ] wizard requires metered warning;
- [ ] wizard shows network state;
- [ ] review summary is correct;
- [ ] Start Tunnel starts service or shows blocked reason.

## 7.9 Acceptance

- [ ] Setup wizard can configure a complete Android offer-mode tunnel.
- [ ] Setup wizard output passes Rust/mobile validation.
- [ ] User can start tunnel from Review step.
- [ ] Any unimplemented wizard feature is unchecked and documented.

---

# Phase 8 — Finish Forwards UI honestly

## 8.1 Forwards list

Implement or uncheck/document:

- [ ] configured forwards list;
- [ ] enabled/disabled state;
- [ ] runtime/listening/paused/error state where available;
- [ ] add action;
- [ ] edit action;
- [ ] delete action;
- [ ] enable/disable action;
- [ ] last error where available.

## 8.2 Forward details/actions

Implement or uncheck/document:

- [ ] local address;
- [ ] local URL;
- [ ] remote forward ID;
- [ ] enabled/disabled;
- [ ] runtime status;
- [ ] last error;
- [ ] copy URL;
- [ ] open browser;
- [ ] test local port, if feasible;
- [ ] edit;
- [ ] disable/enable;
- [ ] delete.

## 8.3 Test Local Port decision

Choose one:

### Option A — implement

- [ ] add local socket/http reachability check;
- [ ] report success/failure;
- [ ] avoid blocking UI thread;
- [ ] test success/failure.

### Option B — defer honestly

- [ ] remove/checklist-uncheck Test Local Port;
- [ ] document reason in TODO/validation notes;
- [ ] do not claim Phase 9 complete.

## 8.4 Tests

Add tests:

- [ ] copy URL produces correct URL;
- [ ] open browser intent is created correctly;
- [ ] disabled forward omitted from runtime config or represented compatibly;
- [ ] last error displayed when present;
- [ ] edit regenerates config;
- [ ] delete regenerates config;
- [ ] disable regenerates config;
- [ ] runtime forward state display is correct where available.

## 8.5 Acceptance

- [ ] Forwards screen supports real local browser/app usage.
- [ ] Forward state matches active runtime config.
- [ ] Unimplemented forward actions are not falsely checked.

---

# Phase 9 — Harden JNI/FFI destroy/dispose and errors

## 9.1 Rust FFI audit

Audit all exported FFI functions for:

- [ ] null handles;
- [ ] invalid pointers;
- [ ] invalid strings;
- [ ] interior NUL;
- [ ] CString failures;
- [ ] panics;
- [ ] double free;
- [ ] use after destroy;
- [ ] stop before start;
- [ ] double stop;
- [ ] normal runtime exit state;
- [ ] error runtime exit state.

## 9.2 Panic boundaries

Required:

- [ ] no panic crosses FFI;
- [ ] `p2ptunnel_destroy_runtime()` is panic-safe or explicitly justified;
- [ ] panic updates/reportable error where feasible;
- [ ] functions return structured failure to Kotlin.

## 9.3 Kotlin bridge lifecycle

Implement:

- [ ] `dispose()` marks bridge disposed;
- [ ] methods check disposed state before native call;
- [ ] calls after dispose fail locally with clear error;
- [ ] runtime handle set to zero after destroy;
- [ ] double dispose safe;
- [ ] missing native library surfaces visible error;
- [ ] invalid native status/log JSON surfaces visible error.

## 9.4 Error reporting

Improve:

- [ ] preserve error strings where possible;
- [ ] avoid generic `unknown error` when specific error exists;
- [ ] redact error details before UI/log display;
- [ ] expose actionable messages to Home/Logs.

## 9.5 Runtime task completion

Update Rust mobile controller:

- [ ] normal daemon completion sets stopped/inactive;
- [ ] error daemon completion sets error/inactive;
- [ ] status reflects actual active state;
- [ ] logs include redacted completion/error event.

## 9.6 Tests

Add tests:

- [ ] destroy panic boundary, where feasible;
- [ ] double dispose safe;
- [ ] calls after dispose return clear error;
- [ ] stop before start safe;
- [ ] double stop safe;
- [ ] invalid identity bytes return error;
- [ ] invalid config path returns error;
- [ ] normal runtime completion changes status to stopped;
- [ ] error runtime completion changes status to error;
- [ ] null handle returns error in Rust FFI tests.

## 9.7 Acceptance

- [ ] Native invalid inputs do not crash process.
- [ ] Kotlin bridge lifecycle is safe.
- [ ] Runtime status does not remain stale after completion.

---

# Phase 10 — Apply explicit lint policy to `p2p-mobile`

## 10.1 Audit workspace lints

Inspect:

```text
Cargo.toml
crates/*/Cargo.toml
crates/p2p-mobile/Cargo.toml
```

Document:

- [ ] workspace lint settings;
- [ ] which crates inherit them;
- [ ] why `p2p-mobile` does or does not inherit them.

## 10.2 Preferred fix

Add to `crates/p2p-mobile/Cargo.toml`:

```toml
[lints]
workspace = true
```

Tasks:

- [ ] fix resulting warnings/errors;
- [ ] do not silence warnings broadly;
- [ ] document any necessary FFI-specific allow.

## 10.3 If exceptions are needed

Use narrow exceptions only:

- [ ] crate-level reason documented;
- [ ] function-level `allow` where possible;
- [ ] no broad suppression hiding real issues;
- [ ] safety comments for unsafe blocks/functions.

## 10.4 Validation

Run:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Tasks:

- [ ] clippy passes;
- [ ] no broad lint suppression added;
- [ ] unsafe FFI exceptions documented.

## 10.5 Acceptance

- [ ] `p2p-mobile` lint policy is explicit.
- [ ] Mobile crate is not silently outside workspace lint discipline.

---

# Phase 11 — Fix fake bridge / DTO mismatch

## 11.1 Audit bridge interfaces

Inspect:

- [ ] `TunnelBridge`;
- [ ] `RustTunnelBridge`;
- [ ] `FakeTunnelBridge`;
- [ ] test-specific bridge fakes;
- [ ] `TunnelRepository.refreshStatus()`;
- [ ] native status/log DTOs.

## 11.2 Fix fake status JSON

Ensure every bridge used by `TunnelRepository` emits native-shaped JSON:

```json
{
  "state": "running",
  "mode": "offer",
  "config_path": "...",
  "last_error": null,
  "started_at_unix_ms": 123,
  "active": true
}
```

Tasks:

- [ ] update `FakeTunnelBridge.getStatusJson()`;
- [ ] update tests relying on old `TunnelStatus` JSON;
- [ ] use `NativeRuntimeStatusDto` consistently;
- [ ] ensure fake logs match `NativeLogEventDto`.

## 11.3 Tests

Add tests:

- [ ] fake bridge status decodes through repository;
- [ ] fake bridge logs decode through repository;
- [ ] malformed fake/native JSON surfaces visible error;
- [ ] no test uses UI model JSON as native JSON unless explicitly testing failure.

## 11.4 Acceptance

- [ ] Test fakes match production native contract.
- [ ] Repository tests exercise the same decode path as production.

---

# Phase 12 — Build/native integration verification

## 12.1 Verify Gradle native tasks

Confirm:

- [ ] `buildRustAndroid` uses `cargo ndk`;
- [ ] target `arm64-v8a`;
- [ ] target `x86_64`;
- [ ] output path is `android/app/src/main/jniLibs`;
- [ ] task fails clearly if `cargo-ndk` missing;
- [ ] `preBuild` or `assembleDebug` depends on native build/verification.

## 12.2 Verify APK contents

Run:

```bash
cd android
./gradlew assembleDebug
unzip -l app/build/outputs/apk/debug/app-debug.apk | grep libp2p_mobile.so
```

Expected:

- [ ] `lib/arm64-v8a/libp2p_mobile.so`;
- [ ] `lib/x86_64/libp2p_mobile.so`.

## 12.3 Docs

Update:

```text
docs/ANDROID_BUILD.md
```

Include:

- [ ] Rust toolchain requirements;
- [ ] Android NDK requirements;
- [ ] `cargo-ndk` install command;
- [ ] supported ABIs;
- [ ] Gradle build command;
- [ ] common failures;
- [ ] how to verify APK contains native libs.

## 12.4 Acceptance

- [ ] `assembleDebug` cannot silently package an APK without native library.
- [ ] APK native library presence is documented and verified.

---

# Phase 13 — Full validation

## 13.1 Rust validation

Run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
```

Tasks:

- [ ] `cargo fmt --check` passes;
- [ ] clippy passes with `-D warnings`;
- [ ] Rust tests pass;
- [ ] no lint warnings are hidden/suppressed broadly.

## 13.2 Android native build

Run:

```bash
cargo ndk \
  -t arm64-v8a \
  -t x86_64 \
  -o android/app/src/main/jniLibs \
  build -p p2p-mobile --release
```

Tasks:

- [ ] native build passes;
- [ ] `arm64-v8a` output exists;
- [ ] `x86_64` output exists.

## 13.3 Android build/tests

Run:

```bash
cd android
./gradlew assembleDebug
./gradlew testDebugUnitTest
```

Tasks:

- [ ] `assembleDebug` passes;
- [ ] unit tests pass;
- [ ] APK contains native libraries.

## 13.4 Connected tests

If emulator/device available:

```bash
cd android
./gradlew connectedDebugAndroidTest
```

Tasks:

- [ ] connected tests pass;
- [ ] if not run, document exact reason.

## 13.5 Manual E2E

Run or document NOT RUN:

- [ ] desktop answer started;
- [ ] Android offer configured from UI;
- [ ] Android tunnel started;
- [ ] Android browser reaches `127.0.0.1:<port>`;
- [ ] remote service response recorded;
- [ ] redacted logs collected.

## 13.6 Validation docs

Update:

```text
docs/ANDROID_VALIDATION.md
```

Include:

- [ ] command;
- [ ] result;
- [ ] environment;
- [ ] date;
- [ ] commit hash;
- [ ] unresolved failures;
- [ ] NOT RUN reasons.

## 13.7 Acceptance

- [ ] Full validation results are current and honest.
- [ ] Failed/unavailable validation is not checked as passed.

---

# Phase 14 — Final acceptance checklist

Do not check these until complete.

## 14.1 Runtime/config/security

- [ ] Android-generated `config.toml` validates through real Rust/mobile validation.
- [ ] TLS CA strategy is implemented and tested.
- [ ] `startOfferWithIdentity()` does not require long-lived plaintext `paths.identity`.
- [ ] `identity.enc` is decrypted and used by runtime startup.
- [ ] No plaintext private identity remains at rest.
- [ ] Private identity import is validated.
- [ ] Canonical public identity is generated/rendered from private identity.
- [ ] Remote authorized key file is populated correctly.
- [ ] Android offer mode reaches native runtime start without config/identity validation failure.

## 14.2 ForegroundService

- [ ] Service calls `startForeground()` promptly.
- [ ] Blocking startup work is off main thread.
- [ ] Service owns runtime start/stop.
- [ ] Duplicate starts do not create duplicate runtimes.
- [ ] STOP during pending startup is safe.
- [ ] Native startup failure leaves clear error state.
- [ ] Stop action releases runtime and unregisters callbacks.
- [ ] No hidden background tunnel is possible.

## 14.3 Network policy

- [ ] Cellular/metered blocked by default.
- [ ] Unknown network blocked always.
- [ ] Startup blocked before native runtime on disallowed networks.
- [ ] Running tunnel pauses/stops on disallowed transition.
- [ ] Resume on unmetered works when enabled.
- [ ] Network Policy UI matches service behavior.
- [ ] Metered/cellular warning required before enabling.

## 14.4 UI

- [ ] Setup wizard creates a valid offer config.
- [ ] Setup wizard shows network state and policy result.
- [ ] Setup wizard validates/writes authorized_keys.
- [ ] Review step supports Save and Start Tunnel.
- [ ] Home shows real runtime status and actionable errors.
- [ ] Forwards add/edit/delete/disable updates active runtime config.
- [ ] Forward details support copy/open/test where feasible.
- [ ] Import/export is functional and safe.
- [ ] Logs show native logs and decode failures safely.

## 14.5 Security/redaction

- [ ] Logs redact private identity material before display.
- [ ] Logs redact MQTT passwords/tokens before display.
- [ ] Logs redact SDP and ICE candidates before display.
- [ ] Diagnostics redact private identity material.
- [ ] Diagnostics redact MQTT passwords/tokens.
- [ ] Diagnostics redact SDP and ICE candidates.
- [ ] Private identity export requires explicit warning.
- [ ] Non-localhost bind requires advanced warning.
- [ ] Generated TOML is safely serialized/escaped.

## 14.6 JNI/FFI/lints

- [ ] No panic crosses FFI.
- [ ] Destroy/dispose paths are safe.
- [ ] Calls after dispose fail clearly.
- [ ] Invalid native inputs do not crash app/process.
- [ ] Native runtime normal completion updates state to stopped.
- [ ] Native runtime error completion updates state to error.
- [ ] `p2p-mobile` has explicit lint policy.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.

## 14.7 Compatibility

- [ ] Android offer connects to desktop Rust answer.
- [ ] Android browser reaches remote service via `127.0.0.1:<port>`.
- [ ] Protocol wire formats unchanged.
- [ ] Desktop Rust tests still pass.
- [ ] E2E validation is documented with exact steps/results.

## 14.8 Build/validation

- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo test --workspace --all-targets` passes.
- [ ] `cargo ndk ... build -p p2p-mobile --release` passes.
- [ ] `./gradlew assembleDebug` passes.
- [ ] APK contains `libp2p_mobile.so` for `arm64-v8a`.
- [ ] APK contains `libp2p_mobile.so` for `x86_64`.
- [ ] `./gradlew testDebugUnitTest` passes.
- [ ] Connected Android tests pass if device/emulator is available, or NOT RUN is documented.

---

# Suggested implementation order

1. [ ] Reset checklist honesty.
2. [ ] Document or rerun real Android offer ↔ desktop answer validation.
3. [ ] Move ForegroundService startup work off main thread.
4. [ ] Fix network policy service/UI consistency.
5. [ ] Redact logs before display/copy/export.
6. [ ] Make generated TOML safe.
7. [ ] Validate duplicate `remoteForwardId`.
8. [ ] Finish or honestly uncheck setup wizard gaps.
9. [ ] Finish or honestly uncheck forwards UI gaps.
10. [ ] Harden JNI/FFI destroy/dispose/error handling.
11. [ ] Add explicit `p2p-mobile` lint policy.
12. [ ] Fix fake bridge status/log DTO shape.
13. [ ] Ensure native runtime clean exit updates state.
14. [ ] Verify Gradle/native APK integration.
15. [ ] Run full validation.
16. [ ] Only then check final acceptance items.
