# Android WebRTC Tunnel Final Patch TODO

## 1. Goal

Apply the final small patch to the Android app before final E2E validation.

This TODO is deliberately narrow. Do not redesign the app. Fix the remaining concrete issues from the latest review.

## 2. Rules

- [ ] Keep Android work on the Android feature branch unless the user explicitly says otherwise.
- [ ] Do not merge to `master` until validation passes.
- [ ] Do not change MQTT signaling wire format.
- [ ] Do not change tunnel frame format.
- [ ] Do not change desktop Rust protocol semantics.
- [ ] Do not add TURN.
- [ ] Do not add VPN/TUN mode.
- [ ] Do not add arbitrary Android remote host/port selection.
- [ ] Do not weaken encrypted identity-at-rest behavior.
- [ ] Do not weaken network policy behavior.
- [ ] Do not weaken log/diagnostic redaction.
- [ ] Do not check off E2E compatibility unless the real Android↔desktop test is run and documented.

---

# Phase 0 — Preserve E2E honesty

## 0.1 Audit current validation docs

Inspect:

```text
docs/ANDROID_VALIDATION.md
ANDROID_FINAL_HARDENING_TODO.md
ANDROID_FINAL_HARDENING_TODO(2).md, if present
docs/memory.md
```

Check all references to:

- [ ] Android offer connects to desktop Rust answer.
- [ ] Android browser reaches remote service via `127.0.0.1:<port>`.
- [ ] manual E2E validation complete.
- [ ] E2E validation documented with exact steps/results.

## 0.2 Correct state

If manual E2E is still not run:

- [ ] keep Android offer ↔ desktop answer unchecked;
- [ ] keep Android browser localhost validation unchecked;
- [ ] keep manual E2E validation unchecked;
- [ ] document `NOT RUN`;
- [ ] include exact reason;
- [ ] include future run steps.

If manual E2E is run:

- [ ] document exact desktop command;
- [ ] document desktop config summary;
- [ ] document Android device/emulator and API level;
- [ ] document Android app build;
- [ ] document broker summary with secrets redacted;
- [ ] document configured forward;
- [ ] document Android browser URL;
- [ ] document response result;
- [ ] document redacted Android/desktop logs;
- [ ] mark E2E items complete only after this evidence exists.

## 0.3 Acceptance

- [ ] No E2E item is checked unless real E2E evidence exists.
- [ ] Validation docs clearly distinguish PASS, FAIL, and NOT RUN.

---

# Phase 1 — Apply workspace lint discipline to `p2p-mobile`

## 1.1 Audit lint configuration

Inspect:

```text
Cargo.toml
crates/p2p-mobile/Cargo.toml
```

Document:

- [ ] root workspace lint policy;
- [ ] current mobile crate lint policy;
- [ ] whether workspace Clippy lints apply to `p2p-mobile`;
- [ ] why `unsafe_code` is allowed.

## 1.2 Implement lint policy

Preferred:

```toml
[lints]
workspace = true
```

If Cargo configuration does not allow the preferred form together with a required `unsafe_code` exception:

- [ ] preserve workspace Clippy lint behavior by the narrowest equivalent means;
- [ ] document why the exception is needed;
- [ ] keep the exception limited to Rust `unsafe_code`;
- [ ] do not suppress `unwrap_used`, `todo`, `dbg_macro`, or warning-level Clippy checks broadly.

## 1.3 Fix lint fallout

Run:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Tasks:

- [ ] fix any new warnings/errors;
- [ ] do not hide warnings with broad `allow`;
- [ ] add safety comments for unsafe blocks/functions if lint policy requires it;
- [ ] keep FFI-specific exceptions narrow and documented.

## 1.4 Acceptance

- [ ] `p2p-mobile` is not silently outside workspace lint discipline.
- [ ] `unsafe_code` exception is documented and narrow.
- [ ] Clippy passes with `-D warnings`.

---

# Phase 2 — Wrap FFI destroy in panic boundary

## 2.1 Audit destroy path

Inspect:

```text
p2ptunnel_destroy_runtime
AndroidTunnelController::stop
RustTunnelBridge.dispose()
```

Document:

- [ ] null handle behavior;
- [ ] Kotlin double-dispose behavior;
- [ ] raw double-destroy limitations;
- [ ] whether `stop()` can panic;
- [ ] whether controller drop can panic;
- [ ] where panic/error can be recorded.

## 2.2 Implement panic-safe destroy

Required:

- [ ] `p2ptunnel_destroy_runtime()` uses `catch_unwind` or existing panic-boundary helper;
- [ ] null handle remains safe;
- [ ] stop/drop panic cannot cross FFI;
- [ ] panic is logged/stored where feasible;
- [ ] Kotlin `dispose()` remains double-call safe;
- [ ] calls after `dispose()` fail locally with clear error.

## 2.3 Tests

Add or update tests:

- [ ] null destroy is safe;
- [ ] Kotlin double dispose is safe;
- [ ] calls after dispose return clear error;
- [ ] stop before dispose is safe;
- [ ] panic boundary is covered or explicitly documented if hard to trigger;
- [ ] invalid native paths do not unwind across FFI.

## 2.4 Acceptance

- [ ] FFI destroy path is panic-safe.
- [ ] No panic can unwind across destroy FFI boundary.

---

# Phase 3 — Fix STOP-during-START lifecycle race

## 3.1 Audit lifecycle flow

Inspect:

```text
TunnelForegroundService.onStartCommand(...)
TunnelForegroundService.startOffer(...)
TunnelForegroundService.doStartOffer(...)
TunnelForegroundService.stopServiceWork(...)
TunnelForegroundService.pause(...)
network callback pause/resume paths
```

Document:

- [ ] where START captures state;
- [ ] where STOP cancels startup;
- [ ] where native `repository.start()` can still be in flight;
- [ ] where Running state is published;
- [ ] whether STOP can arrive while native START is in progress.

## 3.2 Implement deterministic lifecycle protection

Choose one:

### Option A — generation token

- [ ] increment generation on every START;
- [ ] increment generation on every STOP;
- [ ] increment generation on every PAUSE/network-block;
- [ ] START captures generation;
- [ ] after native start returns, START checks generation is still current;
- [ ] if stale, call `repository.stop()` and do not publish Running;
- [ ] STOP wins over older START.

### Option B — serialized actor/state machine

- [ ] all START/STOP/PAUSE events go through one serialized lifecycle queue;
- [ ] no overlapping native start/stop transitions;
- [ ] desired state controls final published state;
- [ ] STOP wins over pending START.

## 3.3 Requirements

- [ ] duplicate START does not run duplicate native starts;
- [ ] STOP during pending START is safe;
- [ ] STOP during native START cannot leave stale Running state;
- [ ] network policy pause during START is safe;
- [ ] start-stop-start still works;
- [ ] notification state matches repository state.

## 3.4 Tests

Add tests:

- [ ] duplicate START is serialized;
- [ ] STOP before native START returns prevents Running state;
- [ ] STOP during artificial delayed native START stops runtime after stale success;
- [ ] network pause during pending START prevents Running state;
- [ ] start-stop-start succeeds;
- [ ] repository status is not stale after cancelled START.

## 3.5 Acceptance

- [ ] STOP wins over in-flight START.
- [ ] Stale START cannot publish Running after STOP/PAUSE.
- [ ] Lifecycle behavior is deterministic.

---

# Phase 4 — Remove or async-fix stale `startAnswer()` path

## 4.1 Audit answer path

Inspect:

```text
ACTION_START_ANSWER handling
TunnelForegroundService.startAnswer()
Setup Wizard mode selection
any Answer-mode UI entry points
```

Document:

- [ ] whether answer mode is supported on Android;
- [ ] whether any UI can trigger answer mode;
- [ ] whether `startAnswer()` performs native start synchronously;
- [ ] whether answer mode is covered by tests.

## 4.2 Choose final behavior

Choose one.

### Option A — remove/dead-code eliminate

- [ ] remove unused `startAnswer()` method;
- [ ] ensure `ACTION_START_ANSWER` returns clear disabled error;
- [ ] update tests.

### Option B — keep disabled safely

- [ ] keep answer mode explicitly disabled;
- [ ] attempted answer start returns redacted actionable error;
- [ ] no native startup occurs;
- [ ] no synchronous blocking path remains.

### Option C — make async-safe

- [ ] implement answer path through same lifecycle-safe async flow as offer;
- [ ] add tests equivalent to offer lifecycle tests.

## 4.3 Acceptance

- [ ] No stale synchronous answer native start path remains.
- [ ] Android v1 answer-mode behavior is explicit and tested.

---

# Phase 5 — Clean config import temp file on all paths

## 5.1 Audit config import

Inspect:

```text
ImportExportViewModel.importConfigContent(...)
ConfigRepository.writeConfigAtomically(...)
ConfigRepository.validate...
```

Find temp files such as:

```text
config-import-candidate.toml
```

## 5.2 Implement cleanup

Required:

- [ ] temp file deleted on successful validation/import;
- [ ] temp file deleted on validation failure;
- [ ] temp file deleted on thrown exception;
- [ ] active config unchanged on validation failure;
- [ ] active config unchanged on exception.

Use `try/finally` or equivalent.

## 5.3 Tests

Add tests:

- [ ] valid import deletes temp file;
- [ ] invalid import deletes temp file;
- [ ] thrown validation error deletes temp file;
- [ ] invalid import does not replace active config;
- [ ] exception path does not replace active config.

## 5.4 Acceptance

- [ ] Config import leaves no stale temp file.
- [ ] Invalid import remains transactional.

---

# Phase 6 — Replace `Thread.sleep()` in tests

## 6.1 Audit tests

Search for:

```text
Thread.sleep
delay(...)
runBlocking
```

in Android unit/instrumentation tests.

Pay special attention to:

```text
ForwardsViewModelTest
service lifecycle tests
network policy tests
```

## 6.2 Replace fixed sleeps

Use one or more:

- [ ] `runTest`;
- [ ] test dispatcher;
- [ ] `advanceUntilIdle`;
- [ ] deterministic fake bridge callback;
- [ ] polling helper with timeout and clear failure;
- [ ] `CompletableDeferred`;
- [ ] fake socket/server lifecycle synchronization.

## 6.3 Tests

- [ ] no `Thread.sleep()` remains in unit tests unless narrowly justified;
- [ ] Test Local Port success/failure tests remain reliable;
- [ ] lifecycle race tests are deterministic;
- [ ] tests do not become timing-flaky.

## 6.4 Acceptance

- [ ] Tests avoid fixed sleeps where deterministic synchronization is possible.
- [ ] Test Local Port tests are stable.

---

# Phase 7 — Clean up Setup Wizard remote public identity UX

## 7.1 Audit current wizard layout

Inspect:

```text
SetupScreen
SetupViewModel
SetupConfigInput
```

Document where these are shown:

- [ ] local private identity import/generation;
- [ ] local public identity;
- [ ] local peer ID;
- [ ] remote peer ID;
- [ ] remote public identity.

## 7.2 Fix labeling/placement

Preferred:

- [ ] local identity generation/import remains on Identity step;
- [ ] local public identity remains on Identity step;
- [ ] remote peer ID moves to Remote Peer step;
- [ ] remote public identity moves to Remote Peer step;
- [ ] Remote Peer step validates peer ID/public identity match;
- [ ] Review step clearly separates Local Identity and Remote Peer.

Acceptable:

- [ ] if not moving fields, clearly label remote identity section as Remote Peer Identity;
- [ ] avoid implying remote public identity belongs to local identity;
- [ ] keep mismatch validation.

## 7.3 Tests

Add/update tests:

- [ ] local identity values shown in Identity step;
- [ ] remote public identity values shown/labeled in Remote Peer step or clearly separated;
- [ ] remote peer mismatch still rejected;
- [ ] review summary clearly separates local and remote peers.

## 7.4 Acceptance

- [ ] Wizard does not confuse local identity with remote peer identity.
- [ ] Existing local/remote peer consistency validation remains intact.

---

# Phase 8 — Keep Test Local Port honest

## 8.1 Verify implementation

Inspect:

```text
ForwardsViewModel.testLocalPort(...)
ForwardsScreen
ForwardsViewModelTest
```

Confirm:

- [ ] probe runs off UI thread;
- [ ] probe targets the configured local host/port;
- [ ] success/failure result is shown;
- [ ] failure is actionable;
- [ ] tests are deterministic after Phase 6.

## 8.2 Acceptance

- [ ] Test Local Port is implemented and tested, or checklist says deferred.
- [ ] There is no false claim about copy/open/test support.

---

# Phase 9 — Validation

## 9.1 Rust validation

Run:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
```

Tasks:

- [ ] `cargo fmt --check` passes;
- [ ] Clippy passes with `-D warnings`;
- [ ] Rust tests pass;
- [ ] no broad lint suppression added.

## 9.2 Android native build

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

## 9.3 Android build/tests

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

## 9.4 Connected tests

If emulator/device available:

```bash
cd android
./gradlew connectedDebugAndroidTest
```

Tasks:

- [ ] connected tests pass;
- [ ] if not run, document exact reason.

## 9.5 APK native library check

Run:

```bash
unzip -l android/app/build/outputs/apk/debug/app-debug.apk | grep libp2p_mobile.so
```

Expected:

- [ ] `lib/arm64-v8a/libp2p_mobile.so`;
- [ ] `lib/x86_64/libp2p_mobile.so`.

## 9.6 Manual E2E

Run if environment is available:

- [ ] start desktop answer;
- [ ] configure Android offer from UI;
- [ ] start Android tunnel;
- [ ] open Android browser at `http://127.0.0.1:<port>`;
- [ ] confirm remote service response;
- [ ] collect redacted logs;
- [ ] document PASS/FAIL.

If not available:

- [ ] document `NOT RUN`;
- [ ] leave E2E acceptance unchecked.

## 9.7 Documentation

Update:

```text
docs/ANDROID_VALIDATION.md
```

Include:

- [ ] date;
- [ ] commit hash;
- [ ] environment;
- [ ] command results;
- [ ] E2E result or NOT RUN reason;
- [ ] unresolved failures.

## 9.8 Acceptance

- [ ] Validation docs are current.
- [ ] PASS/FAIL/NOT RUN are clearly distinguished.
- [ ] No unavailable validation is marked as passing.

---

# Phase 10 — Final acceptance checklist

## 10.1 Required for this final patch

- [ ] `p2p-mobile` inherits workspace Clippy discipline or has narrow documented equivalent.
- [ ] FFI destroy path is panic-safe.
- [ ] STOP during START cannot publish stale Running.
- [ ] stale synchronous answer path removed, disabled safely, or made async-safe.
- [ ] config import temp files are cleaned on success/failure/exception.
- [ ] fixed sleeps removed from tests where practical.
- [ ] Setup Wizard remote public identity UX is clear.
- [ ] Test Local Port implementation/checklist is honest.
- [ ] validation docs are updated.

## 10.2 Required before compatibility acceptance

- [ ] Android offer connects to desktop Rust answer.
- [ ] Android browser reaches remote service via `127.0.0.1:<port>`.
- [ ] E2E result is documented with exact steps/results.

## 10.3 Required before merge

- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo test --workspace --all-targets` passes.
- [ ] `cargo ndk ... build -p p2p-mobile --release` passes.
- [ ] `./gradlew assembleDebug` passes.
- [ ] `./gradlew testDebugUnitTest` passes.
- [ ] connected tests pass if available, or NOT RUN is documented.
