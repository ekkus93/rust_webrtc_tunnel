# Android WebRTC Tunnel Tiny Final Patch TODO

## 1. Goal

Apply the last tiny Android hardening patch before running real Android offer ↔ desktop answer E2E validation.

This is intentionally small. Do not redesign the app. Do not reopen broad Android architecture work.

The patch should fix only:

1. Prevent stale native start after STOP/PAUSE by adding a pre-native-start generation check.
2. Explicitly cancel `startupJob` on STOP/PAUSE paths.
3. Remove `runBlocking` from `SetupViewModel.saveAndApplyConfig()`.
4. Add a clear comment explaining why `p2p-mobile` mirrors workspace Clippy lints instead of using `[lints] workspace = true`.

Manual E2E compatibility should remain unchecked until actually run.

---

## 2. Rules

- [ ] Do not change MQTT signaling wire format.
- [ ] Do not change tunnel frame format.
- [ ] Do not change desktop Rust protocol semantics.
- [ ] Do not add TURN.
- [ ] Do not add VPN/TUN mode.
- [ ] Do not add arbitrary Android remote host/port selection.
- [ ] Do not weaken encrypted identity-at-rest behavior.
- [ ] Do not weaken network policy behavior.
- [ ] Do not weaken log/diagnostic redaction.
- [ ] Do not mark Android↔desktop E2E complete unless the real test is run and documented.

---

# Phase 1 — Add pre-native-start generation check

## 1.1 Audit current lifecycle generation logic

Inspect:

```text
TunnelForegroundService.startOffer(...)
TunnelForegroundService.doStartOffer(...)
TunnelForegroundService.stopServiceWork(...)
TunnelForegroundService.pause(...)
TunnelForegroundService.pauseForPolicy(...)
```

Confirm:

- [ ] where `lifecycleGeneration` increments;
- [ ] where START captures the generation;
- [ ] where STOP increments generation;
- [ ] where PAUSE/network-block increments generation;
- [ ] where `repository.start(...)` is called;
- [ ] where post-start stale-generation check is performed.

## 1.2 Add pre-start stale check

Before calling native startup:

```kotlin
repository.start(...)
```

add a generation/desired-state check.

Required behavior:

- [ ] START captures generation before async startup work begins.
- [ ] Immediately before native `repository.start(...)`, check that captured generation is still current.
- [ ] If stale, do **not** call `repository.start(...)`.
- [ ] If stale, publish no Running state.
- [ ] If stale, leave repository/UI in stopped/paused/blocked state as appropriate.
- [ ] Keep the existing post-start stale check after `repository.start(...)` returns.

Suggested pattern:

```kotlin
val stillCurrentBeforeNativeStart = lifecycleMutex.withLock {
    lifecycleGeneration == startGeneration
}
if (!stillCurrentBeforeNativeStart) {
    return
}

val result = repository.start(...)

val stillCurrentAfterNativeStart = lifecycleMutex.withLock {
    lifecycleGeneration == startGeneration
}
if (!stillCurrentAfterNativeStart) {
    withContext(Dispatchers.IO) { repository.stop() }
    return
}
```

## 1.3 Tests

Add or update tests:

- [ ] STOP before native start prevents `repository.start(...)` from being called.
- [ ] PAUSE/network-block before native start prevents `repository.start(...)` from being called.
- [ ] STOP during native start still stops stale successful runtime.
- [ ] stale START never publishes Running after STOP.
- [ ] start-stop-start still works.

## 1.4 Acceptance

- [ ] STOP/PAUSE before native start prevents native start.
- [ ] STOP/PAUSE during native start prevents stale Running state.
- [ ] Existing lifecycle tests still pass.

---

# Phase 2 — Explicitly cancel `startupJob` on STOP/PAUSE

## 2.1 Audit current job handling

Inspect:

```text
TunnelForegroundService.stopServiceWork(...)
TunnelForegroundService.pause(...)
TunnelForegroundService.pauseForPolicy(...)
TunnelForegroundService.onDestroy(...)
```

Find all paths that currently do:

```kotlin
startupJob = null
```

without first cancelling the active job.

## 2.2 Implement explicit cancellation

Required:

- [ ] On manual STOP, call `startupJob?.cancel()` before clearing it.
- [ ] On manual PAUSE, call `startupJob?.cancel()` before clearing it.
- [ ] On network-policy pause/block, call `startupJob?.cancel()` before clearing it.
- [ ] On service destroy, call `startupJob?.cancel()` before clearing/cancelling service scope.
- [ ] Keep cancellation safe if `startupJob` is already null or completed.
- [ ] Do not block the main thread waiting for cancellation.

Suggested helper:

```kotlin
private fun cancelStartupJob() {
    startupJob?.cancel()
    startupJob = null
}
```

Use it consistently.

## 2.3 Tests

Add or update tests:

- [ ] STOP cancels pending startup job.
- [ ] PAUSE cancels pending startup job.
- [ ] network-policy pause cancels pending startup job.
- [ ] duplicate STOP remains safe.
- [ ] cancellation does not break start-stop-start.

## 2.4 Acceptance

- [ ] No path clears `startupJob` without cancelling it first.
- [ ] Pending startup work is cancelled as early as possible.

---

# Phase 3 — Remove `runBlocking` from `SetupViewModel.saveAndApplyConfig()`

## 3.1 Audit current setup save/start flow

Inspect:

```text
SetupViewModel.saveAndApplyConfig(...)
SetupViewModel.startTunnelFromReview(...)
ConfigRepository.savePreferences(...)
ConfigRepository.preferences
```

Confirm:

- [ ] where config is rendered;
- [ ] where config is validated;
- [ ] where config is written atomically;
- [ ] where preferences are saved;
- [ ] where service start is triggered;
- [ ] whether service start waits for preference save.

## 3.2 Make save operation nonblocking

Remove:

```kotlin
runBlocking { ... }
```

from ViewModel/UI-facing setup code.

Use one of these designs.

### Preferred: suspend save function

- [ ] Make the internal save/apply operation suspendable.
- [ ] Perform preference read/write from coroutine context.
- [ ] Start Tunnel only after save completes successfully.
- [ ] Keep UI responsive.

Example shape:

```kotlin
fun saveAndApplyConfig() {
    viewModelScope.launch {
        saveAndApplyConfigInternal()
    }
}

private suspend fun saveAndApplyConfigInternal(): Result<Unit> {
    ...
}
```

For Start Tunnel:

```kotlin
fun startTunnelFromReview() {
    viewModelScope.launch {
        val saved = saveAndApplyConfigInternal()
        if (saved.isSuccess) {
            startForegroundService(...)
        }
    }
}
```

### Acceptable alternative: callback/result state

- [ ] Save runs in `viewModelScope.launch`.
- [ ] UI state records save success/failure.
- [ ] Start Tunnel chains from successful save.
- [ ] Service start cannot race ahead of preference save.

## 3.3 Preserve behavior

Ensure:

- [ ] generated config still validates before write;
- [ ] config write remains atomic;
- [ ] preferences save completes before service start;
- [ ] errors are shown in setup state;
- [ ] no UI-thread blocking remains;
- [ ] no preference-save race is reintroduced.

## 3.4 Tests

Add or update tests:

- [ ] `saveAndApplyConfig()` does not use `runBlocking`.
- [ ] Start Tunnel waits for preference save.
- [ ] failed config validation prevents service start.
- [ ] failed preference save prevents service start and shows error.
- [ ] successful save starts service exactly once.
- [ ] UI state updates after async save.

## 3.5 Acceptance

- [ ] No `runBlocking` remains in `SetupViewModel` setup save/start path.
- [ ] Start Tunnel still waits for preferences to persist.
- [ ] Setup UI remains responsive.

---

# Phase 4 — Add `p2p-mobile` lint-policy comment

## 4.1 Audit current lint block

Inspect:

```text
crates/p2p-mobile/Cargo.toml
```

Current expected shape may include:

```toml
[lints.rust]
unsafe_code = "allow"

[lints.clippy]
all = { level = "warn", priority = -1 }
dbg_macro = "deny"
todo = "deny"
unwrap_used = "deny"
```

## 4.2 Add explanatory comment

Add a clear comment explaining why `p2p-mobile` does not simply use:

```toml
[lints]
workspace = true
```

if that is still the case.

The comment should say:

- [ ] `p2p-mobile` is the JNI/FFI boundary.
- [ ] Rust `unsafe_code` must be allowed narrowly for JNI/FFI exports.
- [ ] The Clippy lint list intentionally mirrors the workspace policy.
- [ ] The crate must not weaken `unwrap_used`, `todo`, or `dbg_macro`.
- [ ] If workspace lint policy changes, this crate's mirrored Clippy policy must be updated too, unless Cargo config is refactored to inherit workspace lints directly.

Suggested comment:

```toml
# This crate is the Android JNI/FFI boundary and must allow Rust `unsafe_code`
# for exported native functions and pointer handling. Cargo does not let this
# crate inherit the workspace lint table while overriding only `unsafe_code` in
# the shape we need here, so the Clippy policy below intentionally mirrors the
# workspace policy. Keep this list in sync with `[workspace.lints.clippy]`.
```

Adjust wording if Cargo behavior differs.

## 4.3 Validation

Run:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## 4.4 Acceptance

- [ ] The lint exception is documented.
- [ ] Clippy policy remains equivalent to workspace policy for `p2p-mobile`.
- [ ] Clippy passes.

---

# Phase 5 — Validation

## 5.1 Rust validation

Run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
```

Tasks:

- [ ] `cargo fmt --check` passes.
- [ ] Clippy passes with `-D warnings`.
- [ ] Rust tests pass.
- [ ] No broad lint suppression added.

## 5.2 Android validation

Run:

```bash
cargo ndk \
  -t arm64-v8a \
  -t x86_64 \
  -o android/app/src/main/jniLibs \
  build -p p2p-mobile --release

cd android
./gradlew assembleDebug
./gradlew testDebugUnitTest
```

Tasks:

- [ ] native build passes.
- [ ] `assembleDebug` passes.
- [ ] unit tests pass.
- [ ] APK contains native libraries.

## 5.3 Connected tests

If a device/emulator is available:

```bash
cd android
./gradlew connectedDebugAndroidTest
```

Tasks:

- [ ] connected tests pass; or
- [ ] NOT RUN is documented with exact reason.

## 5.4 Manual E2E

If the environment is available, run:

- [ ] desktop answer started;
- [ ] Android offer configured from UI;
- [ ] Android tunnel started;
- [ ] Android browser opens `http://127.0.0.1:<port>`;
- [ ] remote service response confirmed;
- [ ] redacted logs collected;
- [ ] result documented.

If the environment is not available:

- [ ] document `NOT RUN`;
- [ ] leave E2E compatibility unchecked.

## 5.5 Documentation

Update:

```text
docs/ANDROID_VALIDATION.md
```

Include:

- [ ] date;
- [ ] commit hash;
- [ ] environment;
- [ ] command results;
- [ ] connected test result or NOT RUN reason;
- [ ] manual E2E result or NOT RUN reason;
- [ ] unresolved failures.

---

# Phase 6 — Final acceptance checklist

## 6.1 Tiny final patch acceptance

- [ ] Pre-native-start generation check prevents stale native start after STOP/PAUSE.
- [ ] Post-native-start generation check still prevents stale Running publication.
- [ ] STOP/PAUSE paths explicitly cancel `startupJob`.
- [ ] `SetupViewModel` save/start path no longer uses `runBlocking`.
- [ ] Start Tunnel waits for preference save before starting service.
- [ ] `p2p-mobile` lint-policy comment explains the mirrored Clippy policy and unsafe exception.
- [ ] Rust validation passes.
- [ ] Android validation passes.
- [ ] Validation docs are updated.

## 6.2 Compatibility acceptance

Do not check these unless real manual E2E is run:

- [ ] Android offer connects to desktop Rust answer.
- [ ] Android browser reaches remote service via `127.0.0.1:<port>`.
- [ ] E2E result is documented with exact steps/results.
