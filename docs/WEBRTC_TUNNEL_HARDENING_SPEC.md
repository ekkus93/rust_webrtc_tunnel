# WebRTC Tunnel Android/Data-Plane Hardening Spec

## 1. Purpose

This spec defines the next hardening pass for the WebRTC tunnel app after the `webrtc_tunnel-master_2606201346` review. The goal is to make the Android/WebRTC tunnel fail loudly, diagnose accurately, and avoid dangerous fallback behavior that can hide the real cause of connection or data-plane failures.

The core theme is simple:

> A failed primary path must not be disguised as success, an empty result, a default config, or a different transport path.

This work should preserve the current architectural direction:

- STUN-only WebRTC; TURN remains unsupported.
- Rust core owns the tunnel/session logic.
- Android app uses JNI/FFI to start, stop, inspect, and diagnose the native tunnel.
- Android private identity remains encrypted at rest with Android Keystore.
- Android should block metered/cellular operation unless explicitly allowed.
- Data channel readiness must mean real post-DCEP application-level data can flow.

## 2. Non-goals

The following are explicitly out of scope for this pass:

- Adding TURN support.
- Replacing the WebRTC library.
- Rewriting the entire Android UI.
- Implementing a new signaling protocol.
- Adding backward-compatible migration support for unreleased/broken config formats unless it is low risk and explicitly tested.
- Preserving unsafe fallback behavior for convenience.

## 3. Current issues this spec addresses

The latest review found the following high-risk patterns:

1. Android ICE `auto` can fall back from the intended vnet path to native ICE.
2. Android local-IP discovery still uses a hard-coded UDP connect to `8.8.8.8:80`.
3. JNI/native error paths can collapse real failures into `-1` plus `"unknown error"`.
4. JNI status/log/probe calls replace invalid native output with `{}` or `[]`.
5. Log retrieval failure can appear as an empty log list.
6. Corrupt setup draft config silently resets to defaults.
7. Some forwards helpers still convert corrupt config to empty config or no-op.
8. Plaintext identity bytes are not wiped after use.
9. Fake tunnel bridge code exists in production source.
10. Diagnostics overclaim bidirectional data-plane success.
11. Candidate-gather diagnostics can report success even when no useful candidates are gathered.
12. Unknown ICE states and missing ICE candidates are coerced into normal-looking values.
13. Callback channel send failures are ignored too broadly.
14. Unexpected data-channel close can look like a clean shutdown.

## 4. Required behavior

### 4.1 STUN-only policy

TURN must remain unsupported.

Required behavior:

- Any `turn:` or `turns:` ICE server URL must be rejected during **configuration
  validation** (so it fails before tunnel startup), with the WebRTC-construction-time guard
  retained as a second line of defense (replies3).
- Rejection must include a clear error message: `TURN servers are not supported in STUN-only
  mode`. (The current construction guard's `"TURN URLs are not supported in v1"` should be
  aligned to this wording.)
- The app must not silently remove TURN URLs and continue.
- The app must not downgrade to another signaling or relay behavior.

Acceptance criteria:

- Unit tests cover `turn:` and `turns:` rejection.
- Tests verify that invalid TURN config fails before tunnel startup.
- No production config builder silently filters TURN URLs.

### 4.2 Android ICE path selection

Android ICE mode selection must be explicit, observable, and fail-loud.

> **Decision (replies3):** keep the existing four mode names — `native`, `vnet`, `vnet_mux`,
> `auto`. Do **not** rename to `vnet_required`/`native_required`/`auto_strict`/
> `auto_best_effort`; that rename would erase the `mux` distinction, and the `mux` dimension
> is the actual Android fix (`vnet_mux` works; plain `vnet`, with its interface-pinned socket,
> is the broken black-hole). The premise that `auto` means "vnet then native" was inverted:
> the code picks **native** when interface enumeration succeeds. `android_ice_mode` is honored
> on all platforms (no Android branch in the Rust core).

Required behavior:

- Android production builds must not silently use native ICE. The Android default is strict
  `vnet_mux`, set by the **Kotlin layer** (Android-generated configs write
  `android_ice_mode = "vnet_mux"`). The shared Rust core default stays `auto`.
- `auto` is an explicit best-effort/diagnostic mode only. It may select native, but must
  report the selected path and reason; it must never look like the normal successful Android
  path. Hide it from normal Android setup (advanced-only).
- Strict `vnet_mux` must fail loud (visible to Kotlin/UI and native status) if it cannot
  initialize — specifically when no Android-injected local address is available or when UDP
  mux setup fails. It must never continue as native ICE.
- The selected ICE path must be included in status/diagnostics.
- Any fallback/best-effort reason must be included in status/diagnostics.

Mode semantics for this pass:

```text
vnet_mux  Strict Android-safe path. UDP mux + advertise the Android-provided local IPv4. No native fallback.
vnet      Explicit non-mux vnet path. Not the Android default; advanced/diagnostic (not the proven fix).
native    Explicit native ICE. Not the Android default; if selected on Android it must be obvious in status.
auto      Explicit best-effort. Not the Android default. May select native, but must report selected path + reason.
```

Acceptance criteria:

- Android-generated config defaults to strict `vnet_mux`, not `auto`.
- A failed `vnet_mux` setup (no injected address / mux failure) fails startup with a real
  error and never silently continues as native ICE.
- Status must expose at least:
  - requested ICE mode
  - selected ICE path
  - whether fallback/best-effort occurred
  - fallback reason, if any
- Tests cover strict `vnet_mux` failure (does not become native) and best-effort `auto`
  selection separately.

### 4.3 Android local IP/address discovery

Android must not use hard-coded `8.8.8.8:80` local-IP discovery as the production Android path.

Required behavior:

- Android should obtain active-network addresses from Android APIs, preferably `ConnectivityManager` and `LinkProperties`.
- Kotlin should pass selected network/address information to Rust when needed.
- The selected address source must be visible in diagnostics.
- UDP-connect local-IP discovery may remain as a non-Android desktop fallback only.
- If the Android API cannot provide a suitable address, startup/diagnostics must fail with a clear error in strict mode.

Acceptance criteria:

- No Android production path calls a helper that connects UDP to `8.8.8.8:80` for local-IP discovery.
- Diagnostics distinguish `android_link_properties`, `desktop_udp_route_probe`, and `unavailable` address sources.
- Unit tests or Android tests cover no-address and multiple-address cases.

### 4.4 JNI/native error propagation

Every native failure crossing the JNI boundary must preserve the real error message.

Required behavior:

- Native calls must not return only `-1` without storing or returning the correlated error.
- Java string conversion failures, `CString` creation failures, controller lookup failures, native panic boundaries, and controller runtime errors must all produce explicit error messages.
- Kotlin must not receive `unknown error` when the native side had enough context to report the real cause.
- Prefer structured results over integer return codes where practical.

Recommended structured native result:

```json
{
  "ok": false,
  "error": "specific failure message",
  "kind": "validation|jni|runtime|panic|not_running"
}
```

Acceptance criteria:

- Tests force at least one JNI/pre-controller failure and verify a specific error reaches Kotlin.
- `nativeLastError()` is no longer the only way to retrieve failure context for new or revised APIs.
- All nonzero native return paths set a meaningful last error if integer return codes remain.

### 4.5 Native status/log/probe output decoding

Invalid native output must be loud.

Required behavior:

- JNI methods must not replace invalid native UTF-8 or invalid native JSON with `{}` or `[]`.
- Status failure should return an explicit error status object.
- Log failure should return a synthetic error log event or a `Result` failure visible to the UI.
- Probe/diagnostic failure should return explicit diagnostic failure JSON.

Forbidden patterns:

```kotlin
emptyList()
```

```rust
unwrap_or_else(|_| "{}".to_owned())
unwrap_or_else(|_| "[]".to_owned())
```

Required replacement behavior:

```json
{
  "state": "error",
  "last_error": "native returned invalid UTF-8 for status JSON"
}
```

or:

```json
[
  {
    "level": "error",
    "message": "native returned invalid UTF-8 for log JSON"
  }
]
```

Acceptance criteria:

- Tests cover invalid UTF-8/native malformed output and verify the UI/repository sees an error, not empty/default content.
- No production JNI bridge uses `{}` or `[]` as a fallback for decoding failure.

### 4.6 Android repository failure handling

Repository APIs must not disguise failures as empty/default values.

Required behavior:

- `TunnelRepository.recentLogs()` must not return `emptyList()` when native log fetch/decode fails.
- Config loading methods must return `Result`, sealed state, or explicit error objects when config exists but is corrupt.
- Setup draft corruption must be visible to the UI.
- Forwards config corruption must block mutation until resolved, not become empty config.
- Delete/update operations must report failures rather than silently returning.

Acceptance criteria:

- Corrupt setup draft shows a visible error or reset prompt.
- Corrupt forwards config blocks add/update/delete and reports the corruption.
- Log fetch failure appears as an error state or error log item.
- Unit tests cover corrupt JSON files and failed native log parsing.

### 4.7 Identity handling

Private identity handling must minimize plaintext lifetime.

Required behavior:

- Private identity remains encrypted at rest with Android Keystore.
- Plaintext private identity `ByteArray`s must be wiped with `fill(0)` after use.
- Wiping must occur in `finally` blocks so failures do not skip cleanup.
- Import/export validation paths must also wipe plaintext identity buffers.
- Logs and exceptions must not include private key contents.

Acceptance criteria:

- Code paths that call `readPrivateIdentityPlaintext()` wipe the returned buffer.
- Tests or review checks verify `try/finally` cleanup around native start and import validation.
- No log statement includes private key material.

### 4.8 Fake bridge isolation

Fake tunnel implementations must not live in production source.

Required behavior:

- `FakeTunnelBridge` must be moved out of `src/main`.
- Test fakes belong in `src/test`, `src/androidTest`, or `src/debug` depending on intended use.
- Production dependency injection must not be able to select a fake success bridge unless the build variant is explicitly debug/test.

Acceptance criteria:

- Release build source set cannot reference `FakeTunnelBridge`.
- Unit tests still have access to a fake bridge via test fixtures or test source.
- A release build cannot start with a fake native bridge.

### 4.9 Data-plane readiness and diagnostics

Data-channel open is not enough. Success requires application-level bytes to round-trip.

Required behavior:

- Offer side must not start user TCP forwarding until the post-DCEP probe succeeds.
- Probe success must mean bidirectional application-level data flow, not only one-way delivery.
- Answer side may need to start enough bridge/probe handling to reply, but status must distinguish `ProbingDataPlane` from fully ready.
- Diagnostics must not use terms like `echoed` unless the answer actually sends data back and the offer verifies it.

Acceptance criteria:

- Probe protocol verifies ping/pong or equivalent bidirectional round trip.
- Diagnostics text matches actual behavior.
- Status exposes `ProbingDataPlane` or equivalent intermediate state.
- Tests cover probe timeout and verify user forwarding does not start.

### 4.10 Candidate gathering diagnostics

Candidate gathering success must be based on useful candidates, not merely absence of thrown errors.

Required behavior:

- Diagnostics must report counts by candidate type where possible:
  - host
  - srflx
  - relay, expected to be zero in STUN-only mode
  - unknown
- `ok=true` must require at least one useful candidate, not just `error == null`.
- If no candidates are gathered, diagnostics must fail with a clear message.

Acceptance criteria:

- Tests cover `api_ok=true` but zero candidates and verify candidate gathering is not reported as successful.
- UI can show `API worked but no usable candidates were gathered`.

### 4.11 ICE state and candidate handling

Unknown/empty ICE values must be explicit.

Required behavior:

- Unknown upstream ICE states must map to `Unknown` or `Unspecified`, not `New`.
- Missing remote ICE candidate content must be explicitly handled.
- If `None` means end-of-candidates, model it as end-of-candidates.
- If `None` is invalid in this protocol, reject it with an error.

Acceptance criteria:

- Tests cover unknown ICE state mapping.
- Tests cover missing candidate content.
- No production code uses `unwrap_or_default()` to turn a missing ICE candidate into an empty candidate string.

### 4.12 Callback send failures and channel close handling

Dropped internal events must not be ignored during active sessions.

Required behavior:

- Ignoring callback channel send failures is acceptable only during known shutdown/teardown.
- Dropped data/message events during active sessions must be logged or returned as errors.
- Unexpected data-channel close with active streams must be reported distinctly from clean shutdown.

Acceptance criteria:

- Code distinguishes clean shutdown from premature channel close.
- Active-stream close produces a visible error such as `data channel closed while N streams were active`.
- Send failures are logged at appropriate severity unless shutdown is already in progress.

## 5. Logging and status requirements

Native/Android status should expose enough information to debug real failures without reading source code.

Required status fields should include, where applicable:

- tunnel state
- last error
- requested ICE mode
- selected ICE path
- fallback occurred: true/false
- fallback reason
- local address source
- candidate counts
- data-plane probe state
- heartbeat state
- active stream count
- last disconnect reason

Log requirements:

- No private key material.
- No swallowed decode errors.
- No empty-log fallback on failure.
- Warnings for best-effort fallbacks.
- Errors for strict-mode failures.

## 6. Testing requirements

At minimum, add or update tests for:

1. TURN rejection.
2. Android ICE strict mode no-fallback behavior.
3. Android address-source failure behavior.
4. JNI failure preserving a specific error message.
5. Invalid native UTF-8/JSON output becoming explicit error state.
6. Native log failure not becoming empty logs.
7. Corrupt setup config visible to UI/repository.
8. Corrupt forwards config blocks mutation.
9. Identity plaintext buffer wipe around native start/import validation.
10. Fake bridge unavailable from release source set.
11. Bidirectional post-DCEP probe success/failure.
12. Candidate gathering zero-candidate failure.
13. Unknown ICE state mapping.
14. Missing ICE candidate handling.
15. Premature data-channel close while streams are active.

## 7. Manual validation checklist

Before considering this pass complete, manually verify:

- Release build uses the real native bridge only.
- Android default ICE mode is strict/fail-loud.
- Simulated Android local-address failure does not fall back to native ICE.
- Native status screen shows selected ICE path and address source.
- Corrupt setup draft produces visible UI error.
- Corrupt forwards config cannot be overwritten accidentally by add/delete operations.
- Native log failure appears as an error, not as no logs.
- Post-DCEP probe timeout prevents TCP forwarding from starting.
- Data-channel close with active streams is visible as an error.
- Private identity is still encrypted at rest and never logged.

## 8. Definition of done

This hardening pass is complete when:

- All P0 and P1 TODO items are implemented.
- Relevant P2 cleanup is either implemented or explicitly deferred with justification.
- Test suite covers the high-risk failure modes above.
- No reviewed production code path converts failure/corruption into silent defaults.
- No Android production path uses hard-coded `8.8.8.8` local-IP discovery.
- No Android default path falls back to native ICE without explicit opt-in.
- Data-plane readiness means verified bidirectional application-level bytes.
