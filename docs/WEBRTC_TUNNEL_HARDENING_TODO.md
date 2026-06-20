# WebRTC Tunnel Android/Data-Plane Hardening TODO

Priority scale:

- `P0`: release blocker / can hide critical failure or violate core constraints.
- `P1`: high-priority hardening / likely to cause confusing failures or unsafe behavior.
- `P2`: important cleanup / improves correctness, diagnostics, and maintainability.
- `P3`: optional polish / useful but not required for this pass.

## P0 tasks

### P0-001 — Remove silent Android ICE fallback-to-native behavior

Files likely involved:

- `crates/p2p-webrtc/src/lib.rs`
- Android config/model files that expose `android_ice_mode`
- Native status/diagnostic structs
- Kotlin setup/settings UI, if ICE mode is user-visible

Problem:

> **Premise correction (replies3):** The earlier description ("`auto` tries vnet then
> native") was inverted. The actual `decide_ice_path` (`p2p-webrtc/src/lib.rs`) picks
> **native** ICE when OS interface enumeration *succeeds*, and only falls back to
> `vnet_mux` when enumeration *fails*. `android_ice_mode` is honored on **all** platforms
> — there is no Android-vs-desktop branch in the Rust core. So the dangerous case is
> `auto` choosing **native** when enumeration succeeds, which on Android is the proven
> black-hole path.

`android_ice_mode=auto` can select native ICE on Android (whenever interface enumeration
succeeds), which is the path we proved black-holes offer→answer data on the A54. This
disguises the real Android ICE failure and makes later WebRTC/data-plane issues much harder
to debug.

Required changes:

- **Keep the current four mode names** (`native`, `vnet`, `vnet_mux`, `auto`). Do **not**
  rename to `vnet_required`/`native_required`/`auto_strict`/`auto_best_effort` in this pass
  — the rename would drop the critical `mux` distinction (`vnet_mux` is the proven Android
  fix; plain `vnet` is the broken pinned-socket path).
- **Make the Android default strict `vnet_mux`, owned by the Kotlin layer.** Android-generated
  configs write `android_ice_mode = "vnet_mux"`. Do **not** change the Rust core default by
  `cfg(target_os = "android")` — the Rust default stays `auto` for the shared desktop/CLI core.
- Treat `auto` as an explicit best-effort/diagnostic mode. Hide it from normal Android setup
  (advanced-only, with a warning). If `auto` selects native on Android, that must be visible
  in status — never presented as the normal successful Android path.
- Rust/mobile still fails loud in strict modes (defense-in-depth): if `vnet_mux` is requested
  and no Android-injected local address is available, or mux setup fails, fail startup with a
  specific error. Never continue as native ICE from `vnet_mux`.
- Include requested mode, selected path, fallback boolean, and fallback reason in
  status/diagnostics.

Acceptance criteria:

- Android-generated config defaults to `vnet_mux`; no setup/reset/default path writes `auto`.
- Strict `vnet_mux` fails startup with a specific error when address injection or mux setup
  fails; it never silently continues as native ICE.
- `auto` remains available as explicit best-effort; if it selects native, status shows it.
- Unit tests cover strict `vnet_mux` failure and best-effort `auto` selection separately.

---

### P0-002 — Replace Android `8.8.8.8:80` local-IP discovery

> **Coupled with P0-001 (replies3):** strict `vnet_mux` needs a real local IPv4 to advertise
> as its host candidate. Today that address comes from the `8.8.8.8` probe via
> `fallback_net()`. Removing the probe on Android therefore requires the Android-injected
> address to land **in the same patch set** — P0-002 is not separable from P0-001. A
> short-lived interim (remove `8.8.8.8`, make `vnet_mux` fail loud without an injected
> address) is acceptable only if it is not claimed as complete; the done state requires
> address injection.

Files likely involved:

- `crates/p2p-webrtc/src/lib.rs`
- `crates/p2p-mobile/src/diagnostics.rs`
- Android Kotlin network policy/diagnostics layer
- JNI/native config bridge

Problem:

Android production code still uses a UDP connect to `8.8.8.8:80` to infer the local IP. This is brittle, hides Android network-selection details, and bypasses `ConnectivityManager` / `LinkProperties`.

Required changes:

- Use Android `ConnectivityManager` and `LinkProperties` to obtain active-network link addresses.
- Pass the selected IPv4/local address or address set to Rust as needed.
- Keep UDP-route probing only for non-Android desktop fallback if still useful.
- Report the address source in diagnostics/status.

Acceptance criteria:

- Android production path does not call hard-coded `8.8.8.8:80` discovery.
- Diagnostics report address source as `android_link_properties`, `desktop_udp_route_probe`, or explicit failure.
- No-address Android case fails loudly in strict mode.
- Tests cover missing active network/address and multiple-address selection.

---

### P0-003 — Fix JNI/native error propagation so real failures are preserved

Files likely involved:

- `crates/p2p-mobile/src/lib.rs`
- `crates/p2p-mobile/src/jni_bridge.rs`
- `android/app/src/main/java/com/phillipchin/webrtctunnel/data/RustTunnelBridge.kt`
- `android/app/src/main/java/com/phillipchin/webrtctunnel/data/TunnelRepository.kt`

Problem:

Some native calls return integer error codes such as `-1` without guaranteeing that a correlated detailed error is available to Kotlin. Kotlin can receive `unknown error`, especially for failures before controller execution.

Required changes:

- Ensure every native error path stores or returns a specific error message.
- Include Java string conversion failures, `CString` failures, invalid controller handle, native runtime errors, and panic boundaries.
- Prefer structured native result JSON for revised APIs.
- If integer return codes remain, guarantee `nativeLastError()` is meaningful after any nonzero return.

Acceptance criteria:

- Tests force JNI/pre-controller errors and verify specific messages reach Kotlin.
- Kotlin no longer reports `unknown error` when native has failure context.
- All nonzero native control returns have correlated error text.

---

### P0-004 — Stop replacing invalid native status/log/probe output with `{}` or `[]`

Files likely involved:

- `crates/p2p-mobile/src/jni_bridge.rs`
- Android bridge/repository decode paths
- Status/log/probe model classes

Problem:

JNI bridge code currently uses fallback strings like `{}` or `[]` when native output cannot be decoded as UTF-8. This hides native corruption/FFI bugs as empty or default status/log output.

Required changes:

- Replace `{}` fallback with explicit error status JSON.
- Replace `[]` fallback with an explicit synthetic error log entry or `Result` failure.
- Make malformed JSON visible to repository/UI.
- Remove all production `unwrap_or_else(|_| "{}"...)` and `unwrap_or_else(|_| "[]"...)` patterns around native output.

Acceptance criteria:

- Invalid UTF-8 status returns a visible error status.
- Invalid UTF-8 logs return an error log item or failure state.
- Tests cover invalid native output and assert it does not become empty/default content.

---

### P0-005 — Ensure data-plane readiness means bidirectional application-level bytes

Files likely involved:

- `crates/p2p-tunnel/src/probe.rs`
- `crates/p2p-daemon/src/offer/session/mod.rs`
- `crates/p2p-daemon/src/answer/session/*`
- diagnostics/status code

Problem:

DataChannel open is not enough. The app must verify real post-DCEP application-level bytes before starting user forwarding. Diagnostics currently overclaim echo behavior in at least one path.

> **Scope collapse (replies3):** the core is already done. `crates/p2p-tunnel/src/probe.rs`
> already performs a bidirectional `Ping`→`Pong` round trip, the offer already waits for the
> matching `Pong` before starting user TCP forwarding, and the mid-session self-heal
> heartbeat already exists. **Do not rebuild the probe.** This item narrows to the missing
> status state, the missing one-way-failure test, and diagnostics wording.

Required changes (narrowed):

- Add/verify an answer-side `ProbingDataPlane` status state so the answer UI/status does not
  imply full user-forwarding readiness before the probe completes.
- Add a one-way-only failure test: offer→answer delivery succeeds but no `Pong` returns, and
  the offer must not start user TCP forwarding.
- Tighten diagnostics wording so it only says `echo`/`round trip`/`bidirectional` when a
  `Pong`/echo was actually received and verified.
- Keep the existing bidirectional probe and heartbeat/self-heal behavior; do not replace them.

Acceptance criteria:

- Existing bidirectional session probe remains in place; offer-side forwarding still does not
  start until the matching `Pong` returns.
- A one-way-only data path fails the probe and prevents user TCP forwarding.
- Answer-side status distinguishes probe handling (`ProbingDataPlane`) from fully ready.
- Diagnostics wording is precise and does not overclaim.

---

## P1 tasks

### P1-001 — Make `TunnelRepository.recentLogs()` fail visibly instead of returning `emptyList()`

Files likely involved:

- `android/app/src/main/java/com/phillipchin/webrtctunnel/data/TunnelRepository.kt`
- log screen/view model/UI
- tests for repository/log parsing

Problem:

Native log retrieval or decode failure can become an empty log list. That makes the UI look like there are no logs instead of showing that log retrieval failed.

Required changes:

- Change `recentLogs()` to return `Result<List<LogEvent>>`, a sealed state, or include a synthetic error log event.
- UI must show the failure.
- Do not use `getOrDefault(emptyList())` for native log failures.

Acceptance criteria:

- Failed native log fetch/decode produces visible error state or error log item.
- Tests cover the failure path.
- Empty list is used only for a successful log fetch with no logs.

---

### P1-002 — Remove forwards config helpers that treat corruption as empty config

Files likely involved:

- `android/app/src/main/java/com/phillipchin/webrtctunnel/data/ForwardsConfigStore.kt`
- forwards repository/view model/UI
- forwards config tests

Problem:

`loadForwards()` still converts corrupt forwards config into `emptyList()`, and delete/update paths can silently no-op on corrupt config.

Required changes:

- Delete `loadForwards()` or change it to return `Result<List<ForwardRule>>`.
- Make delete/update/add return explicit failure when baseline config is corrupt.
- Preserve existing corrupt file until user explicitly resets or repairs it.
- Ensure no mutation accidentally overwrites corrupt config with empty/default config.

Acceptance criteria:

- Corrupt forwards config blocks add/update/delete.
- User/repository receives explicit corruption error.
- Tests prove corrupt config is not overwritten by empty config.

---

### P1-003 — Make corrupt setup draft visible instead of resetting to defaults

Files likely involved:

- `android/app/src/main/java/com/phillipchin/webrtctunnel/data/ConfigRepository.kt`
- setup UI/view model
- setup config tests

Problem:

`loadSetupInput()` returns `SetupConfigInput()` when saved setup draft JSON is corrupt. That silently resets fields and hides corruption.

Required changes:

- Replace with `loadSetupInputResult()` or sealed state.
- UI should show a clear error with options to reset or repair/re-enter values.
- Do not overwrite corrupt draft unless user explicitly resets or saves new config.

Acceptance criteria:

- Corrupt setup draft produces visible error.
- Tests cover corrupt setup JSON.
- No default setup value is returned for existing corrupt setup file.

---

### P1-004 — Wipe plaintext private identity byte arrays after use

Files likely involved:

- `android/app/src/main/java/com/phillipchin/webrtctunnel/service/TunnelForegroundService.kt`
- `android/app/src/main/java/com/phillipchin/webrtctunnel/data/ImportExportService.kt`
- identity repository callers

Problem:

Plaintext private identity bytes are read into Kotlin `ByteArray`s and not wiped after use.

Required changes:

- Wrap plaintext identity use in `try/finally`.
- Call `identity.fill(0)` in `finally`.
- Apply this to native startup, import validation, and any export/validation path that handles plaintext private identity.
- Ensure logs/exceptions never include key material.

Acceptance criteria:

- Every `readPrivateIdentityPlaintext()` caller wipes the returned byte array.
- Cleanup occurs even when native start/import validation throws.
- Tests or code review checks cover the key paths.

---

### P1-005 — Move `FakeTunnelBridge` out of production source

Files likely involved:

- `android/app/src/main/java/com/phillipchin/webrtctunnel/data/RustTunnelBridge.kt`
- Android test/debug source sets
- dependency injection setup

Problem:

A fake success bridge exists in `src/main`. It is not currently wired by default, but keeping it in production source is a future footgun.

Required changes:

- Move `FakeTunnelBridge` to `src/test`, `src/androidTest`, or `src/debug`.
- Ensure release builds cannot reference or instantiate it.
- Keep tests working through a test fixture or test-only fake.

Acceptance criteria:

- `FakeTunnelBridge` is absent from release/main source set.
- Release build uses only the real native bridge.
- Tests still have a fake bridge available from test/debug source.

---

### P1-006 — Make candidate gathering diagnostics require useful candidates

Files likely involved:

- `crates/p2p-mobile/src/diagnostics.rs`
- diagnostics result models/UI
- diagnostics tests

Problem:

Candidate gather diagnostic success can be based only on absence of errors, even if no useful candidates were gathered.

Required changes:

- Report candidate counts by type: host, srflx, relay, unknown.
- Use separate fields if needed:
  - `api_ok`
  - `candidate_gathering_ok`
- Set candidate gathering success to false when no useful candidates are present.

Acceptance criteria:

- Zero-candidate case is diagnostic failure, not success.
- UI can explain `API call succeeded but no usable candidates were gathered`.
- Tests cover zero candidates and useful candidate cases.

---

### P1-007 — Surface selected ICE path, fallback reason, local address source, and candidate counts in Android status

Files likely involved:

- Rust status structs
- JNI status serialization
- Kotlin status models
- Android status UI

Problem:

Even when logs contain warnings, the Android app needs structured status fields so users can see what path is actually running.

Required changes:

- Add status fields for requested ICE mode, selected ICE path, fallback reason, local address source, and candidate counts.
- Display these fields in diagnostics/status UI.
- Ensure fields are populated during startup failure as well as successful startup.

Acceptance criteria:

- User can tell whether native/vnet ICE is active without reading logs.
- User can see why fallback happened if best-effort mode is explicitly enabled.
- Tests verify status serialization/deserialization for the new fields.

---

## P2 tasks

### P2-001 — Add explicit `Unknown`/`Unspecified` ICE state mapping

Files likely involved:

- `crates/p2p-webrtc/src/lib.rs`
- ICE state enums/models/tests

Problem:

Unknown upstream ICE states are currently mapped to `New`, which can misrepresent unexpected states as normal startup.

Required changes:

- Add `Unknown` or `Unspecified` state.
- Map unmapped upstream states to that explicit state.
- Log or surface unknown state as diagnostic warning.

Acceptance criteria:

- Unknown upstream state does not become `New`.
- Tests cover unknown state mapping.

---

### P2-002 — Handle missing ICE candidate content explicitly

Files likely involved:

- `crates/p2p-webrtc/src/lib.rs`
- signaling candidate model/tests

Problem:

`candidate.candidate.unwrap_or_default()` turns missing candidate content into an empty string.

Required changes:

- If missing candidate means end-of-candidates, model it explicitly.
- If missing candidate is invalid, return a clear error.
- Remove `unwrap_or_default()` for ICE candidate content.

Acceptance criteria:

- Tests cover missing candidate behavior.
- Empty candidate string is not accidentally passed into WebRTC as a real candidate.

---

### P2-003 — Log callback channel send failures unless shutdown is expected

Files likely involved:

- `crates/p2p-webrtc/src/lib.rs`
- data-channel callback handling
- tunnel/session code that sends through channels

Problem:

Several callback/event sends use `let _ = ...`. Some are harmless during teardown, but dropped message events during active sessions can hide real tunnel failures.

Required changes:

- Audit all ignored send results.
- Keep ignored results only where shutdown is known/expected and comment why.
- Log `warn` for dropped message/data events during active sessions.
- Log `debug` for expected teardown sends.

Acceptance criteria:

- No unexplained `let _ = tx.send(...)` remains for active-session data paths.
- Dropped message events are visible in logs/status where appropriate.

---

### P2-004 — Distinguish clean data-channel close from premature active-stream close

Files likely involved:

- `crates/p2p-tunnel/src/multiplex/offer.rs`
- `crates/p2p-tunnel/src/multiplex/answer.rs`
- session status/error models

Problem:

Data-channel close can return `Ok(())` even when streams may be active. That makes unexpected disconnects look clean.

Required changes:

- Track active stream count at close time.
- Return/report a distinct error if the data channel closes while streams are active or opening.
- Preserve clean shutdown behavior for intentional stop.

Acceptance criteria:

- Active-stream close reports an error like `data channel closed while N streams were active`.
- Intentional stop remains clean.
- Tests cover both cases.

---

### P2-005 — Tighten diagnostics wording around one-way versus bidirectional tests

Files likely involved:

- `crates/p2p-mobile/src/diagnostics.rs`
- Android diagnostics UI strings
- diagnostics tests

Problem:

Some diagnostics say bytes were `echoed` even when the code only proved one-way delivery.

Required changes:

- Rename one-way tests to `offer-to-answer delivery` or equivalent.
- Use `echo`/`round trip` only for true bidirectional tests.
- Prefer implementing true echo and removing one-way-only success language.

Acceptance criteria:

- Diagnostic text accurately describes what was tested.
- Tests verify bidirectional echo if the diagnostic claims echo.

---

### P2-006 — Add explicit tunnel lifecycle state for data-plane probing

Files likely involved:

- Rust session status models
- JNI status serialization
- Kotlin status models/UI

Problem:

Answer side may need to start enough handling to respond to a probe before the offer side considers the tunnel ready. Status should not imply full user-forwarding readiness too early.

Required changes:

- Add state such as `ProbingDataPlane`.
- Use it between DataChannel open and probe completion.
- Show probe timeout/failure distinctly.

Acceptance criteria:

- UI can distinguish connected-but-probing from ready-for-user-traffic.
- Probe failure is visible as probe failure, not generic disconnect.

---

### P2-007 — Add regression tests for no-silent-fallback policy

Files likely involved:

- Rust unit/integration tests
- Android local unit tests
- Android instrumentation tests where feasible

Problem:

The project has multiple historical patterns where failures become defaults. This needs a targeted regression suite.

Required changes:

Add tests for:

- corrupt setup draft does not become defaults
- corrupt forwards config does not become empty config
- native invalid status/log output does not become `{}` / `[]`
- strict Android ICE failure does not become native ICE
- zero candidates does not become diagnostic success
- log fetch failure does not become empty logs

Acceptance criteria:

- Tests fail if any of the old fallback behavior is reintroduced.
- Test names clearly describe the no-silent-fallback rule.

---

## P3 tasks

### P3-001 — Add a developer-facing diagnostics summary command/log block

Files likely involved:

- native diagnostics
- Android diagnostics UI/export

Goal:

Add a compact support bundle that reports:

- app version/build variant
- requested/selected ICE path
- local address source
- candidate counts
- data-plane probe result
- heartbeat result
- last disconnect reason
- redacted config summary

Acceptance criteria:

- User can copy/share diagnostics without exposing private key material.
- Sensitive fields are redacted.

---

### P3-002 — Add comments documenting allowed fallback behavior

Files likely involved:

- WebRTC config builder
- Android network/address selection
- JNI output conversion
- config repositories

Goal:

Document where fallback is allowed and where fail-loud behavior is required.

Acceptance criteria:

- Best-effort paths are clearly named and documented.
- Strict production paths explain why fallback is forbidden.

---

# Suggested implementation order

> Per replies3: `P0-001` and `P0-002` are implemented **together** (strict `vnet_mux` needs
> the Android-injected address), and `P0-005` is collapsed to status/wording/one-way-test.

1. `P0-001` + `P0-002` together — Android default strict `vnet_mux` + `ConnectivityManager`/
   `LinkProperties` address injection (drop `8.8.8.8` on the Android path).
3. `P0-003` JNI/native error propagation.
4. `P0-004` native output decode failures become explicit errors.
5. `P0-005` bidirectional data-plane probe semantics.
6. `P1-001` log retrieval failure visibility.
7. `P1-002` forwards corruption handling.
8. `P1-003` setup corruption handling.
9. `P1-004` identity byte wiping.
10. `P1-005` move fake bridge out of production.
11. `P1-006` useful-candidate diagnostics.
12. `P1-007` Android status fields.
13. P2 cleanup and regression coverage.

# Final acceptance gate

Before this TODO is considered complete, verify all of the following:

- No Android default path silently falls back from vnet/strict ICE to native ICE.
- No Android production path uses hard-coded `8.8.8.8` for local-IP discovery.
- Every native failure crossing JNI has a specific user-visible or repository-visible error.
- Native decode failures are not represented as `{}`, `[]`, defaults, or empty logs.
- Corrupt config files are visible failures and are not overwritten accidentally.
- Plaintext private identity buffers are wiped after use.
- Release source set contains no fake tunnel bridge.
- Data-plane readiness requires bidirectional app-level bytes.
- Candidate diagnostics require useful candidates.
- Regression tests exist for the no-silent-fallback cases.
