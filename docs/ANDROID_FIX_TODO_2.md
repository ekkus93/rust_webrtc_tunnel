# Android WebRTC Tunnel Fix TODO 2

## 1. Goal

Fix the remaining Android app blockers after the previous Android fix pass.

This TODO is implementation-oriented. Do not treat UI polish as complete until the native/runtime/config/security path is proven.

The highest-priority outcome is:

```text
Android-generated config validates through Rust.
Encrypted Android identity is actually used by native runtime startup.
No long-lived plaintext private identity exists.
Android offer connects to desktop Rust answer.
Android browser reaches remote service through 127.0.0.1:<port>.
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
- [ ] Do not check off any acceptance item unless the implementation and validation are done.

---

# Phase 0 — Correct previous checklist and validation honesty

## 0.1 Uncheck premature completion claims

Audit:

```text
ANDROID_WEBRTC_TUNNEL_TODO.md
ANDROID_FIX_TODO1.md
docs/memory.md
docs/ANDROID_VALIDATION.md
```

Uncheck or annotate any item that is not currently proven, especially:

- [ ] Android-generated config validates against Rust.
- [ ] `identity.enc` is used by actual tunnel startup.
- [ ] no plaintext private identity remains at rest.
- [ ] setup wizard is truly functional.
- [ ] forwards add/edit/delete update active runtime config.
- [ ] Android offer connects to desktop Rust answer.
- [ ] Android browser reaches remote service through localhost.
- [ ] validation commands pass.

## 0.2 Record current validation state

Run or document inability to run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo ndk -t arm64-v8a -t x86_64 -o android/app/src/main/jniLibs build -p p2p-mobile --release
cd android && ./gradlew assembleDebug
cd android && ./gradlew testDebugUnitTest
cd android && ./gradlew connectedDebugAndroidTest
```

- [ ] Add exact results to `docs/ANDROID_VALIDATION.md`.
- [ ] If a command cannot be run, document why.
- [ ] If a command fails, include the failing command and concise failure summary.
- [ ] Do not mark validation complete until the command actually passes.

---

# Phase 1 — Fix Android config / Rust config compatibility

## 1.1 Audit current Rust config schema

Inspect:

```text
crates/p2p-core/src/config.rs
crates/p2p-mobile/src/*
android/app/src/main/java/**/ConfigRepository.kt
```

Document:

- [ ] required top-level config fields;
- [ ] required `paths.*` fields;
- [ ] required `broker.*` fields;
- [ ] required `broker.tls.*` fields;
- [ ] required files/directories that must exist;
- [ ] how `forwards` are represented;
- [ ] whether `broker.tls.ca_file` is required for `mqtts://`.

## 1.2 Choose TLS CA strategy

Pick exactly one strategy.

### Preferred: make Rust support default/native root store

- [ ] Change Rust config to allow `broker.tls.ca_file` to be optional where safe.
- [ ] Update validation so `mqtts://` does not require `ca_file` when the TLS stack can use default roots.
- [ ] Preserve desktop compatibility with existing configs.
- [ ] Document behavior in Android docs.

### Alternative: bundle an Android CA bundle

- [ ] Add a real CA bundle asset or generated app-private CA file.
- [ ] Ensure the file exists before Rust validation.
- [ ] Set `broker.tls.ca_file` to that actual path.
- [ ] Document update/security implications.

### Alternative: user-imported CA file

- [ ] Add UI/import flow for CA file.
- [ ] Disable `mqtts://` start until CA is provided.
- [ ] Show actionable error if CA is missing.

## 1.3 Fix Android config renderer

Update `ConfigRepository` so generated `config.toml`:

- [ ] contains only app-private Android paths;
- [ ] contains no `~/.config`;
- [ ] contains no `~/.local`;
- [ ] contains no hardcoded `/etc/ssl/certs`;
- [ ] contains valid TLS config according to chosen strategy;
- [ ] contains valid authorized keys path;
- [ ] contains valid state/runtime directories;
- [ ] contains correct offer-mode forwards;
- [ ] does not expose arbitrary remote host/port on Android offer side.

## 1.4 Add atomic config rendering

Implement atomic config writes:

- [ ] render to temp file;
- [ ] validate temp file;
- [ ] atomically replace active `config.toml` only if valid;
- [ ] leave previous config unchanged if validation fails;
- [ ] surface validation failure in UI.

## 1.5 Tests

Add tests:

- [ ] default Android config contains no desktop paths;
- [ ] generated Android config includes valid TLS strategy;
- [ ] generated Android config validates through Rust/mobile validation;
- [ ] invalid generated config is rejected with actionable error;
- [ ] config write is atomic;
- [ ] failed validation does not replace previous config.

## 1.6 Acceptance

- [ ] A config produced by the Android setup flow is accepted by real Rust/mobile config validation.
- [ ] A config produced by Android can be used by tunnel startup without manual editing.
- [ ] TLS CA behavior is explicit, implemented, and tested.

---

# Phase 2 — Fix in-memory identity startup

## 2.1 Audit current identity startup path

Inspect:

```text
IdentityRepository
TunnelForegroundService
RustTunnelBridge
crates/p2p-mobile
crates/p2p-core config loading
```

Document:

- [ ] where `identity.enc` is stored;
- [ ] where private identity is decrypted;
- [ ] how identity bytes reach JNI;
- [ ] where Rust parses identity;
- [ ] whether Rust still requires `paths.identity` file;
- [ ] whether any plaintext private identity file is created.

## 2.2 Implement preferred identity override

Preferred implementation:

- [ ] Add Rust/mobile API to validate/load config with identity override.
- [ ] When identity override is supplied, do not require `paths.identity` to exist.
- [ ] Parse private identity from supplied bytes.
- [ ] Use parsed identity for runtime startup.
- [ ] Keep desktop config validation unchanged unless explicitly refactored safely.
- [ ] Return clear error if identity bytes are invalid.

Suggested API shape:

```text
validate_config_with_identity(config_path, identity_toml_bytes)
start_offer_with_identity(config_path, identity_toml_bytes)
```

## 2.3 Temporary fallback only if necessary

If in-memory identity override is not feasible:

- [ ] Decrypt identity to a short-lived app-private temp file.
- [ ] Use restrictive file mode.
- [ ] Point runtime validation/startup at the temp path.
- [ ] Delete temp file immediately after Rust loads it.
- [ ] Delete temp file again on stop/error.
- [ ] Delete stale temp files at app/service startup.
- [ ] Never include temp identity path or contents in diagnostics/logs.
- [ ] Document this as temporary technical debt.

## 2.4 Tests

Add tests:

- [ ] startup with `identity.enc` succeeds without long-lived plaintext `paths.identity`;
- [ ] invalid encrypted identity produces actionable error;
- [ ] missing encrypted identity produces setup-required error;
- [ ] no plaintext `identity.toml` remains in `filesDir`;
- [ ] no plaintext private identity remains in `cacheDir`;
- [ ] stop/error cleanup removes temp identity if fallback strategy is used;
- [ ] diagnostics do not include identity bytes or temp identity paths.

## 2.5 Acceptance

- [ ] `identity.enc` is actually used by native runtime startup.
- [ ] Rust startup no longer requires a long-lived plaintext private identity file.
- [ ] Android offer can reach native runtime start after config validation.

---

# Phase 3 — Fix private/public identity import, export, and generation

## 3.1 Add Rust-backed identity helpers if possible

Expose mobile-safe Rust helpers:

```text
generate_private_identity() -> private identity TOML
validate_private_identity(private_identity_toml) -> ok/error
render_public_identity(private_identity_toml) -> public identity string
validate_public_identity(public_identity) -> ok/error
```

Tasks:

- [ ] Add Rust implementations or expose existing identity logic.
- [ ] Add JNI bindings.
- [ ] Add Kotlin bridge methods.
- [ ] Surface errors as structured/actionable messages.

## 3.2 Fix private identity import

Update import flow:

- [ ] read selected private identity file;
- [ ] validate with Rust helper;
- [ ] render canonical public identity with Rust helper;
- [ ] encrypt private identity to `identity.enc`;
- [ ] write canonical public identity to `identity.pub`;
- [ ] discard plaintext bytes;
- [ ] never log file contents.

Do not infer public identity by copying a `peer_id` line.

## 3.3 Add/gate identity generation

If Rust helper exists:

- [ ] add Generate Identity action in setup wizard;
- [ ] encrypt generated private identity to `identity.enc`;
- [ ] write `identity.pub`;
- [ ] show public identity to user.

If generation is not available:

- [ ] clearly disable/hide Generate Identity;
- [ ] show import-required message.

## 3.4 Private identity export warning

Implement explicit warning dialog:

```text
Private Identity Export Warning

Anyone with this file can impersonate this phone in your tunnel network.

Only export it if you understand the risk.

[Cancel]
[Export Private Identity]
```

Tasks:

- [ ] require explicit confirmation for every private export;
- [ ] optionally require device unlock/biometric if easy;
- [ ] do not use a passive checkbox as the only warning;
- [ ] export only after successful decrypt.

## 3.5 Tests

Add tests:

- [ ] valid private identity import writes `identity.enc`;
- [ ] valid private identity import writes canonical `identity.pub`;
- [ ] invalid private identity import is rejected;
- [ ] empty private identity import is rejected;
- [ ] public identity export matches canonical Rust format;
- [ ] private export requires warning confirmation;
- [ ] no plaintext private identity remains after import/export.

## 3.6 Acceptance

- [ ] Imported private identity is validated before storage.
- [ ] Public identity is canonical and usable by desktop peer.
- [ ] Private identity export is explicitly warned.
- [ ] No plaintext private identity persists at rest.

---

# Phase 4 — Fix authorized_keys / remote peer import

## 4.1 Audit authorized key format

Inspect Rust desktop/daemon expectations:

- [ ] public identity line format;
- [ ] peer ID requirement;
- [ ] authorized key file location;
- [ ] multiple peer behavior;
- [ ] per-forward authorization behavior, if applicable.

## 4.2 Implement remote public identity import

Setup wizard Remote Peer step must support:

- [ ] paste remote public identity;
- [ ] import remote public identity file;
- [ ] validate public identity with Rust helper if available;
- [ ] write valid identity to `filesDir/authorized_keys`;
- [ ] avoid duplicate entries;
- [ ] show remote peer ID in Review step.

## 4.3 Tests

Add tests:

- [ ] valid remote public identity is accepted;
- [ ] invalid remote public identity is rejected;
- [ ] authorized_keys is created if missing;
- [ ] duplicate remote identity is not duplicated;
- [ ] generated config points at app-private authorized_keys.

## 4.4 Acceptance

- [ ] Android offer can authorize the desktop answer peer using `authorized_keys`.
- [ ] Remote peer identity import is not a placeholder.

---

# Phase 5 — Unify forwards source of truth

## 5.1 Choose source-of-truth model

Preferred:

```text
Structured Android config state -> render config.toml atomically
```

Tasks:

- [ ] identify current structured state files/preferences;
- [ ] decide where forwards are stored;
- [ ] document the source of truth in code comments/docs;
- [ ] ensure tunnel start always uses config rendered from current state.

## 5.2 Regenerate active config on forward mutation

For every forward action:

- [ ] add;
- [ ] edit;
- [ ] delete;
- [ ] enable;
- [ ] disable;

do:

- [ ] update structured state;
- [ ] render candidate `config.toml`;
- [ ] validate candidate config;
- [ ] atomically replace active config if valid;
- [ ] rollback state or show error if validation fails.

## 5.3 Validate forwards

Rules:

- [ ] port must be 1-65535;
- [ ] duplicate enabled local ports rejected;
- [ ] duplicate forward IDs rejected;
- [ ] local host defaults to `127.0.0.1`;
- [ ] non-localhost bind requires advanced warning;
- [ ] Android offer side does not expose arbitrary remote host/port;
- [ ] disabled forwards are either omitted from Rust config or marked in a Rust-compatible way.

## 5.4 Tests

Add tests:

- [ ] add forward updates active config;
- [ ] edit forward updates active config;
- [ ] delete forward updates active config;
- [ ] disable forward updates active config;
- [ ] duplicate local port rejected;
- [ ] duplicate forward ID rejected;
- [ ] non-localhost requires warning;
- [ ] generated local URL is correct;
- [ ] runtime start uses updated config.

## 5.5 Acceptance

- [ ] Forwards UI changes affect the actual tunnel runtime config.
- [ ] There is no stale `config.toml` after forward edits.

---

# Phase 6 — Make config import/export transactional and Android-safe

## 6.1 Config import

Implement:

- [ ] read candidate config from selected file/path;
- [ ] write candidate to temp file;
- [ ] validate candidate through Rust/mobile validation;
- [ ] if valid, atomically replace active config;
- [ ] if invalid, keep previous active config;
- [ ] show actionable validation error.

## 6.2 Config export

Implement:

- [ ] export current config through Android-safe share/file mechanism if available;
- [ ] redact secrets if export is diagnostics-style;
- [ ] if raw config export includes secrets, show warning;
- [ ] document whether config export is raw or redacted.

## 6.3 Tests

Add tests:

- [ ] invalid config import does not replace active config;
- [ ] valid config import replaces active config;
- [ ] config import reports validation errors;
- [ ] config export does not include private identity;
- [ ] config export behavior around MQTT secrets is explicit and tested.

## 6.4 Acceptance

- [ ] Import cannot leave the app with a broken active config.
- [ ] Export behavior is documented and safe.

---

# Phase 7 — Make network policy consistent and service-enforced

## 7.1 Refine network status model

Represent:

```text
networkType
isMetered
allowedByDefault
allowedByUserPolicy
blockedReason
```

Tasks:

- [ ] update `NetworkPolicyManager`;
- [ ] update UI models;
- [ ] update service logic to use the same policy calculation;
- [ ] fail safe on unknown network;
- [ ] fail safe on no network.

## 7.2 Startup gate

Before native runtime start:

- [ ] load `allowMetered`;
- [ ] load `resumeOnUnmetered`;
- [ ] read current network status;
- [ ] if disallowed, do not start Rust;
- [ ] show paused/blocked notification;
- [ ] update repository/UI state with blocked reason.

## 7.3 Runtime pause/resume

When network changes:

- [ ] if running and network becomes disallowed, stop/pause Rust runtime;
- [ ] close local listeners if runtime supports it;
- [ ] update notification;
- [ ] update UI state;
- [ ] if network becomes allowed and resume is enabled, restart runtime after config/identity checks.

## 7.4 Metered/cellular warning

Implement warning dialog:

```text
Cellular / Metered Data Warning

WebRTC Tunnel can use a large amount of data. Browser traffic, API calls, SSH sessions, downloads, streaming, llama-server usage, or other forwarded traffic may consume your mobile data plan quickly.

Your carrier may charge overage fees, throttle your connection, or suspend service depending on your plan.

The app developer is not responsible for carrier charges, throttling, overage fees, or data-plan exhaustion caused by your use of this feature.

Only enable this if you understand the risk and accept responsibility for any data usage or charges.

[Cancel]
[I understand — allow cellular/metered tunnels]
```

Tasks:

- [ ] require warning acceptance before enabling metered/cellular;
- [ ] store acceptance;
- [ ] show current policy clearly in Settings and Setup wizard;
- [ ] make UI status reflect `allowMetered`.

## 7.5 Tests

Add tests:

- [ ] startup blocked on cellular by default;
- [ ] startup blocked on metered Wi-Fi by default;
- [ ] startup blocked on unknown network by default;
- [ ] explicit allow permits metered/cellular;
- [ ] warning required before enabling metered/cellular;
- [ ] running tunnel pauses on switch to cellular;
- [ ] paused tunnel resumes on unmetered when configured;
- [ ] Network Policy UI reflects allowed status correctly.

## 7.6 Acceptance

- [ ] Network policy is enforced by the service.
- [ ] Network policy UI matches service behavior.
- [ ] Tunnel cannot use cellular/metered data by default.

---

# Phase 8 — Finish Setup Wizard

## 8.1 Choose Mode step

- [ ] Offer mode enabled and default.
- [ ] Answer mode disabled or marked Advanced/Incomplete if not supported.
- [ ] User cannot proceed with unsupported mode unless intentionally allowed.

## 8.2 Identity step

- [ ] Generate identity if Rust API exists.
- [ ] Import private identity.
- [ ] Validate private identity.
- [ ] Store private identity encrypted.
- [ ] Display public identity.
- [ ] Copy public identity.
- [ ] Share/export public identity.
- [ ] Show setup-required error if identity missing.

## 8.3 MQTT Broker step

Fields:

- [ ] broker host;
- [ ] port;
- [ ] TLS enabled;
- [ ] CA strategy/import if required;
- [ ] username optional;
- [ ] password optional;
- [ ] topic prefix optional if supported.

Validation:

- [ ] host required;
- [ ] port valid;
- [ ] TLS settings valid;
- [ ] secrets not logged.

## 8.4 Remote Peer step

Fields/actions:

- [ ] remote peer ID;
- [ ] remote public identity;
- [ ] paste;
- [ ] import file;
- [ ] validate identity;
- [ ] write authorized_keys.

## 8.5 Forwards step

- [ ] add forward;
- [ ] edit forward;
- [ ] remove forward;
- [ ] disable forward;
- [ ] local host default `127.0.0.1`;
- [ ] local port;
- [ ] remote forward ID;
- [ ] no remote target host/port;
- [ ] duplicate validation.

## 8.6 Network Policy step

- [ ] show actual current network state;
- [ ] show blocked/allowed status;
- [ ] keep metered/cellular blocked by default;
- [ ] warning before enabling metered/cellular;
- [ ] show resume-on-unmetered option.

## 8.7 Review step

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

Start Tunnel behavior:

- [ ] save config atomically;
- [ ] validate config;
- [ ] check identity;
- [ ] check network policy;
- [ ] start ForegroundService if allowed;
- [ ] show actionable error if blocked.

## 8.8 Tests

Add tests:

- [ ] cannot proceed from invalid step;
- [ ] wizard creates valid config;
- [ ] wizard writes authorized_keys;
- [ ] wizard stores identity encrypted;
- [ ] wizard rejects duplicate forwards;
- [ ] wizard requires metered warning;
- [ ] review summary is correct;
- [ ] Start Tunnel from Review starts service or shows blocked reason.

## 8.9 Acceptance

- [ ] Setup wizard can configure a complete Android offer-mode tunnel.
- [ ] Setup wizard output passes Rust/mobile validation.
- [ ] User can start tunnel from wizard Review step.

---

# Phase 9 — Finish Forwards UI and behavior

## 9.1 Forwards list

Implement:

- [ ] configured forwards list;
- [ ] enabled/disabled state;
- [ ] runtime/listening/paused/error state where available;
- [ ] add action;
- [ ] edit action;
- [ ] delete action;
- [ ] enable/disable action.

## 9.2 Forward details

Show:

- [ ] local address;
- [ ] local URL;
- [ ] remote forward ID;
- [ ] enabled/disabled;
- [ ] runtime status;
- [ ] last error if available.

Actions:

- [ ] copy URL;
- [ ] open browser;
- [ ] test local port if feasible;
- [ ] edit;
- [ ] disable/enable;
- [ ] delete.

## 9.3 Tests

Add tests:

- [ ] copy URL produces correct `http://127.0.0.1:<port>` URL;
- [ ] open browser intent is created correctly;
- [ ] disabled forward is not active in runtime config;
- [ ] last error is displayed when present;
- [ ] edit/delete/disable regenerate config.

## 9.4 Acceptance

- [ ] Forwards screen is useful for real local browser/app usage.
- [ ] Forward state matches actual runtime config.

---

# Phase 10 — Strengthen logs and diagnostics redaction

## 10.1 Redaction targets

Redact from logs and diagnostics:

- [ ] private identity files;
- [ ] `sign.private`;
- [ ] `kex.private`;
- [ ] private key PEM blocks if ever present;
- [ ] MQTT password;
- [ ] password file contents;
- [ ] bearer tokens;
- [ ] API keys;
- [ ] SDP blobs;
- [ ] ICE candidates;
- [ ] decrypted payloads;
- [ ] forwarded data;
- [ ] temporary private identity paths if temp-file strategy is used.

## 10.2 Diagnostics content

Diagnostics may include only redacted/safe data:

- [ ] status JSON;
- [ ] redacted config;
- [ ] recent redacted logs;
- [ ] network state;
- [ ] app version;
- [ ] Rust/mobile library version;
- [ ] device/API level if useful;
- [ ] validation results if available.

## 10.3 Logs screen

Implement/verify:

- [ ] All/Debug/Info/Warn/Error filters;
- [ ] copy visible logs;
- [ ] clear logs;
- [ ] export diagnostics;
- [ ] native logs shown;
- [ ] malformed native log JSON surfaces visible error.

## 10.4 Tests

Add tests with realistic multiline examples:

- [ ] private identity TOML redacted;
- [ ] MQTT password redacted;
- [ ] bearer token redacted;
- [ ] SDP redacted;
- [ ] ICE candidate redacted;
- [ ] forwarded data marker redacted;
- [ ] diagnostics do not include identity bytes;
- [ ] diagnostics do not include password strings;
- [ ] native logs are redacted before display/export.

## 10.5 Acceptance

- [ ] Diagnostics and logs are safe to share.
- [ ] Redaction tests cover realistic secret formats.

---

# Phase 11 — Harden ForegroundService lifecycle

## 11.1 Startup

- [ ] Service calls `startForeground()` promptly on every start path.
- [ ] Long-running Rust startup work is not done on main thread.
- [ ] `START_NOT_STICKY` retained unless explicitly justified.
- [ ] null intents handled safely.
- [ ] config/identity/network checks happen before native start.

## 11.2 Stop/cleanup

On stop:

- [ ] native runtime stop is idempotent;
- [ ] network callbacks unregistered;
- [ ] service coroutines cancelled;
- [ ] repository/service state updated;
- [ ] notification updated or removed;
- [ ] foreground stopped;
- [ ] service stops itself.

## 11.3 Tests

Add/update tests:

- [ ] service start posts notification promptly;
- [ ] null intent does not crash;
- [ ] stop action stops runtime;
- [ ] start-stop-start works;
- [ ] no `ForegroundServiceDidNotStartInTimeException`;
- [ ] native startup failure leaves clear error state.

## 11.4 Acceptance

- [ ] Service lifecycle is reliable under Android foreground-service rules.
- [ ] No hidden background tunnel is possible.

---

# Phase 12 — Harden JNI/FFI safety and error reporting

## 12.1 Rust FFI audit

Audit all exported functions for:

- [ ] null handles;
- [ ] invalid pointers;
- [ ] invalid UTF-8 / invalid strings;
- [ ] interior NUL in strings;
- [ ] CString creation failures;
- [ ] panics;
- [ ] double free;
- [ ] use after destroy;
- [ ] stop before start;
- [ ] double stop.

## 12.2 Panic boundaries

- [ ] no panic crosses FFI;
- [ ] wrap exported functions in panic-catching helper;
- [ ] update last error on panic;
- [ ] return structured failure to Kotlin.

## 12.3 Kotlin native availability

- [ ] store native library load success/failure;
- [ ] expose load error to repository/UI;
- [ ] do not swallow `System.loadLibrary()` failures;
- [ ] avoid native calls after bridge dispose/destroy.

## 12.4 Tests

Add tests where feasible:

- [ ] missing native library surfaces visible error;
- [ ] invalid config path returns error, not crash;
- [ ] stop before start safe;
- [ ] double stop safe;
- [ ] invalid identity bytes return error;
- [ ] malformed status/log JSON visible;
- [ ] null handle returns error in Rust FFI tests.

## 12.5 Acceptance

- [ ] Native failures become actionable app errors.
- [ ] Invalid native inputs do not crash the app/process.

---

# Phase 13 — Rust Android library Gradle integration

## 13.1 Verify build tasks

Ensure:

- [ ] `buildRustAndroid` uses `cargo ndk`;
- [ ] targets `arm64-v8a`;
- [ ] targets `x86_64`;
- [ ] outputs to `android/app/src/main/jniLibs`;
- [ ] fails clearly if `cargo-ndk` missing;
- [ ] `preBuild` or `assembleDebug` depends on native build or verification.

## 13.2 Verify APK contents

After build:

- [ ] APK contains `lib/arm64-v8a/libp2p_mobile.so`;
- [ ] APK contains `lib/x86_64/libp2p_mobile.so`;
- [ ] APK does not silently build without native libs.

Command:

```bash
unzip -l android/app/build/outputs/apk/debug/app-debug.apk | grep libp2p_mobile.so
```

## 13.3 Documentation

Update Android build docs:

```bash
cargo install cargo-ndk
cd android
./gradlew assembleDebug
```

Document:

- [ ] required Rust toolchain;
- [ ] required Android NDK;
- [ ] required cargo-ndk;
- [ ] supported ABIs;
- [ ] expected native library output.

## 13.4 Acceptance

- [ ] `./gradlew assembleDebug` produces an APK with native Rust library included.
- [ ] Build failure is clear when native dependencies are missing.

---

# Phase 14 — Protocol compatibility and end-to-end validation

## 14.1 Desktop answer setup

Document exact desktop command, for example:

```bash
cargo run --bin p2p-answer -- --config <desktop-answer-config>
```

Tasks:

- [ ] create/identify desktop answer config;
- [ ] identify answer peer public identity;
- [ ] identify required MQTT broker settings;
- [ ] identify remote forward IDs.

## 14.2 Android offer setup

Using only Android UI:

- [ ] import/generate Android identity;
- [ ] import desktop answer public identity;
- [ ] configure MQTT broker;
- [ ] configure forward `127.0.0.1:8080 -> llama` or equivalent;
- [ ] keep cellular/metered blocked unless intentionally tested;
- [ ] save config;
- [ ] start tunnel.

## 14.3 Browser validation

On Android:

```text
http://127.0.0.1:8080
```

Tasks:

- [ ] confirm remote service responds;
- [ ] record response type/status;
- [ ] record network type;
- [ ] record Android device/emulator;
- [ ] record relevant redacted logs.

## 14.4 Protocol invariants

Verify no changes to:

- [ ] MQTT topic layout;
- [ ] signaling envelope;
- [ ] encrypted inner message schema;
- [ ] identity/public key format;
- [ ] authorized key semantics;
- [ ] tunnel frame format;
- [ ] `OpenPayload { forward_id }`;
- [ ] per-forward authorization.

## 14.5 Documentation

Add results to:

```text
docs/ANDROID_VALIDATION.md
```

Include:

- [ ] date;
- [ ] git commit;
- [ ] desktop command;
- [ ] Android config summary;
- [ ] network type;
- [ ] result;
- [ ] known failures if any.

## 14.6 Acceptance

- [ ] Android offer connects to desktop Rust answer.
- [ ] Android browser reaches remote service through `127.0.0.1:<port>`.
- [ ] Protocol wire formats remain compatible.

---

# Phase 15 — Full validation

## 15.1 Rust validation

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
- [ ] no lint warnings are suppressed to hide real issues.

## 15.2 Android native build

Run:

```bash
cargo ndk \
  -t arm64-v8a \
  -t x86_64 \
  -o android/app/src/main/jniLibs \
  build -p p2p-mobile --release
```

Tasks:

- [ ] native library builds for `arm64-v8a`;
- [ ] native library builds for `x86_64`;
- [ ] outputs are present in `jniLibs`.

## 15.3 Android build/tests

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

## 15.4 Connected tests

If emulator/device available:

```bash
cd android
./gradlew connectedDebugAndroidTest
```

Tasks:

- [ ] connected tests pass;
- [ ] if not run, document why.

## 15.5 Validation note

Update:

```text
docs/ANDROID_VALIDATION.md
```

Include:

- [ ] exact commands;
- [ ] pass/fail result;
- [ ] environment;
- [ ] date;
- [ ] commit hash;
- [ ] unresolved failures.

---

# Phase 16 — Final acceptance checklist

Do not check these until complete.

## 16.1 P0 runtime/config/security

- [ ] Android-generated `config.toml` validates through real Rust/mobile validation.
- [ ] TLS CA strategy is implemented and tested.
- [ ] `startOfferWithIdentity()` does not require long-lived plaintext `paths.identity`.
- [ ] `identity.enc` is decrypted and used by runtime startup.
- [ ] No plaintext private identity remains at rest.
- [ ] Private identity import is validated.
- [ ] Canonical public identity is generated/rendered from private identity.
- [ ] Remote authorized key file is populated correctly.
- [ ] Android offer mode reaches native runtime start without config/identity validation failure.

## 16.2 Network/service

- [ ] ForegroundService starts notification promptly.
- [ ] ForegroundService owns runtime start/stop.
- [ ] Cellular/metered blocked by default.
- [ ] Unknown network blocked by default.
- [ ] Startup blocked before native runtime on disallowed networks.
- [ ] Running tunnel pauses/stops on transition to disallowed network.
- [ ] Resume on unmetered works when enabled.
- [ ] Stop action releases runtime and unregisters callbacks.

## 16.3 UI

- [ ] Setup wizard creates a valid offer config.
- [ ] Review step supports Save and Start Tunnel.
- [ ] Home shows real runtime status and actionable errors.
- [ ] Forwards add/edit/delete/disable updates active runtime config.
- [ ] Forward details support copy/open/test where feasible.
- [ ] Network Policy screen reflects user preferences.
- [ ] Import/export is functional and safe.
- [ ] Logs show native logs and decode failures.

## 16.4 Security

- [ ] Diagnostics redact private identity material.
- [ ] Diagnostics redact MQTT passwords/tokens.
- [ ] Diagnostics redact SDP and ICE candidates.
- [ ] Logs redact secrets.
- [ ] Private identity export requires explicit warning.
- [ ] Non-localhost bind requires advanced warning.

## 16.5 Compatibility

- [ ] Android offer connects to desktop Rust answer.
- [ ] Android browser reaches remote service via `127.0.0.1:<port>`.
- [ ] Protocol wire formats unchanged.
- [ ] Desktop Rust tests still pass.

## 16.6 Validation

- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo test --workspace --all-targets` passes.
- [ ] `cargo ndk ... build -p p2p-mobile --release` passes.
- [ ] `./gradlew assembleDebug` passes.
- [ ] `./gradlew testDebugUnitTest` passes.
- [ ] Connected Android tests pass if present and device/emulator is available.
- [ ] End-to-end validation is documented.

---

# Suggested implementation order

1. [ ] Correct checklist honesty.
2. [ ] Fix Android/Rust config validation and TLS CA strategy.
3. [ ] Fix in-memory identity startup.
4. [ ] Add real Rust/mobile validation tests.
5. [ ] Fix private identity import/public identity rendering.
6. [ ] Fix authorized_keys/remote public identity import.
7. [ ] Unify forwards source of truth.
8. [ ] Make config import transactional.
9. [ ] Make network policy service/UI consistent.
10. [ ] Finish setup wizard.
11. [ ] Finish forwards details.
12. [ ] Strengthen diagnostics/log redaction.
13. [ ] Harden ForegroundService lifecycle.
14. [ ] Harden JNI/FFI errors.
15. [ ] Verify Gradle/native build integration.
16. [ ] Run end-to-end Android offer ↔ desktop answer test.
17. [ ] Run full validation.
18. [ ] Only then check final acceptance items.
