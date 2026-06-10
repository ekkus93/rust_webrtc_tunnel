# P5 — Real Per-Forward Runtime Status TODO

Implements `P5_FORWARD_RUNTIME_STATUS_SPEC.md`. P5 was deferred from the UIUX2
follow-up because the native runtime could not report per-forward status honestly.
This task wires a real daemon → controller status channel (reusing the existing
`DaemonStatus`), replaces fabricated mobile status fields, and adds honest
per-forward `Listening`/`Stopped`/`Error` state for the offer role.

Work in phases; each phase is independently shippable. Do not leak secrets into
status JSON, the status file, logs, or UI. Do not enable Android answer mode.

---

## P0 — Regression guard & decisions before editing

- [ ] Run current tests and record any pre-existing failures:
  - [ ] `cargo test -p p2p-daemon`
  - [ ] `cargo test -p p2p-mobile`
  - [ ] `cd android && ./gradlew --no-daemon testDebugUnitTest lintDebug`
- [ ] Resolve open decisions from the spec (§ "Open decisions"):
  - [ ] **D1 — bind soft-fail vs fatal** (changes CLI startup semantics). Default:
    soft-fail per forward; daemon-level error only if zero forwards bind.
  - [ ] **D2 — status primitive**: confirm `tokio::sync::watch`.
  - [ ] **D3 — per-forward active-connection count**: defer (Listening only) unless
    decided otherwise.
  - [ ] **D5 — `DaemonStatus` seed**: seed the channel with a config-derived
    "starting" status (avoid forcing `Default` on core types).
- [ ] Do not hide or suppress new lint/test failures — fix them.

---

## Phase 1 — Daemon → controller status channel (no schema change)

**Files:** `crates/p2p-daemon/src/{lib.rs,status.rs}`,
`crates/p2p-mobile/src/{runtime.rs,lib.rs}`, CLI crate (delegation only).

**Daemon tasks:**
- [ ] Define `pub type DaemonStatusSink = tokio::sync::watch::Sender<DaemonStatus>;`
      in `p2p-daemon` (or a small `StatusEmitter` wrapping `StatusWriter` + optional
      sink).
- [ ] Thread an optional sink through `RuntimeContext` (`lib.rs:256-260`).
- [ ] At the single `write_status_or_log` choke point, after writing the file, also
      `sink.send(status.clone())` when present (so channel == file).
- [ ] Add `run_offer_daemon_with_status(config, identity, keys, sink)` and
      `run_answer_daemon_with_status(...)`.
- [ ] Make existing `run_offer_daemon` / `run_answer_daemon` delegate with no sink
      so the **CLI is unchanged**.

**Mobile tasks:**
- [ ] In `AndroidTunnelController::start` (`runtime.rs:146`): create a
      `watch::channel` seeded with a config-derived "starting" `DaemonStatus`; store
      the `Receiver` in `RuntimeInner`; pass the `Sender` to the `*_with_status`
      entry point.
- [ ] Rewrite `status()` (`runtime.rs:278`) to derive `AndroidRuntimeStatus` from
      the latest `DaemonStatus` (`rx.borrow()`), merged with controller-owned
      lifecycle facts (`config_path`, `started_at_unix_ms`, terminal `Error`).
- [ ] Map `DaemonState` → `AndroidRuntimeState` honestly (Idle/WaitingForLocalClient/
      Serving → running; Negotiating/ConnectingDataChannel → connecting; TunnelOpen
      → connected; IceRestarting/Renegotiating/Backoff → reconnecting; Closed →
      stopped). Document mapping in code.
- [ ] Surface real `mqtt_connected`, `active_session_count`, `session_capacity` in
      the status JSON emitted by `p2p-mobile/src/lib.rs`.

**Kotlin tasks:**
- [ ] `TunnelRepository.toTunnelStatus()`: set `mqttConnected`,
      `activeSessionCount`, `sessionCapacity` from the now-real native fields
      instead of `active` (`TunnelRepository.kt:160-161`).

**Tests:**
- [ ] Rust: sink receives a `DaemonStatus` clone equal to what was written.
- [ ] Rust: `status()` reflects channel values (fresh vs. running); secret-safe.
- [ ] Kotlin: `mqttConnected`/`activeSessionCount`/`sessionCapacity` reflect decoded
      values, not a hardcoded flag.

**Acceptance:**
- [ ] Home reflects real MQTT/session/connection state within the poll interval.
- [ ] `mqttConnected` is no longer "task spawned".
- [ ] CLI status-file behavior unchanged.

---

## Phase 2 — Per-forward runtime state in the daemon (offer role)

**Files:** `crates/p2p-daemon/src/{status.rs,lib.rs}`.

**Tasks:**
- [ ] Add `ForwardListenState { Listening, #[default] Stopped, Error }`
      (`#[serde(rename_all = "snake_case")]`) to `status.rs`.
- [ ] Add `ForwardRuntimeStatus { id, listen_state, last_error: Option<String> }`.
- [ ] Add `pub forwards: Vec<ForwardRuntimeStatus>` to `DaemonStatus`
      (`status.rs:9-20`); update `new`/`with_sessions` constructors.
- [ ] Maintain a `forward_id -> ForwardRuntimeStatus` map in `DaemonRuntimeState`,
      threaded through `RuntimeContext`, included in every built `DaemonStatus`
      (`write_daemon_status`/`write_answer_status`, `lib.rs:2052-2081`).
- [ ] Populate at `bind_offer_listeners` (`lib.rs:1992-2011`):
  - [ ] bind success → `Listening`.
  - [ ] stop/steady-state teardown → `Stopped`.
  - [ ] bind failure → `Error` + redacted reason (**requires D1 soft-fail**).
- [ ] Answer role: report forwards as `Stopped`/omit; document the limitation.
- [ ] Ensure `last_error` and all forward fields are secret-free.

**Tests:**
- [ ] `DaemonStatus` serializes `forwards` as an array; empty by default.
- [ ] Update schema tests (`current_status_schema_exposes_only_stable_public_fields`
      and `open_forward_ids` assertions) to include `forwards` **without** removing
      secret-absence checks.
- [ ] Successful bind → `Listening`; simulated bind failure → `Error` with a
      non-secret message (per D1).

**Acceptance:**
- [ ] `Listening` means a local TCP listener is actually bound — never derived from
      task spawn.
- [ ] One forward can be `Error` while others are `Listening` (per D1).

---

## Phase 3 — Surface per-forward status to Android

**Files:** `crates/p2p-mobile/src/{runtime.rs,lib.rs}`,
`android/.../model/Models.kt`, `android/.../data/TunnelRepository.kt`,
`android/.../ui/screens.kt`.

**Mobile tasks:**
- [ ] Add `AndroidForwardRuntimeStatus` and
      `forwards: Vec<AndroidForwardRuntimeStatus>` to `AndroidRuntimeStatus`,
      snake_case JSON. Include config-derived `name`, `local_host`, `local_port`,
      `remote_forward_id`, `enabled` plus runtime `listen_state`, `last_error`.
- [ ] Derive these from `DaemonStatus.forwards` joined with the loaded config.

**Kotlin tasks:**
- [ ] Add `NativeRuntimeForwardStatusDto` (see spec §3.2).
- [ ] Add `val forwards: List<NativeRuntimeForwardStatusDto> = emptyList()` to
      `NativeRuntimeStatusDto` (defaulted → backward compatible).
- [ ] Map native forward DTOs → `TunnelStatus.forwards` (`ForwardStatus`) in
      `toTunnelStatus()`.
- [ ] Tolerant listen-state mapper (`listening`/`stopped`/`error`/`disabled`/
      `paused` → enum; unknown → `Stopped`, or `Error` if `last_error` present).
- [ ] Redact `last_error` via `SensitiveDataRedactor` before storing.

**UI tasks:**
- [ ] Keep the `Configured`/`Disabled` label only as a fallback for forwards with
      no runtime entry; otherwise render the real state via
      `forwardStatusChipColors()`.
- [ ] Confirm the Phase B policy-pause safeguard still holds with forwards present.

**Tests:**
- [ ] Kotlin: decode JSON **with** `forwards` → populated; **without** → empty list.
- [ ] Kotlin: unknown `listen_state` does not crash.
- [ ] Rust: `p2p-mobile` status JSON includes `forwards`; fresh → `[]`; running →
      populated; secret-safe.

**Acceptance:**
- [ ] Running tunnel shows real per-forward `Listening`/`Error`/`Stopped`.
- [ ] Disabled forwards still show `Disabled` from config.
- [ ] Older native JSON without `forwards` still decodes.

---

## Phase 4 — Validation gate

```bash
cargo test -p p2p-daemon
cargo test -p p2p-mobile
cargo test
cd android
./gradlew --no-daemon lintDebug
./gradlew --no-daemon testDebugUnitTest
./gradlew --no-daemon assembleDebug
```

- [ ] `cargo test -p p2p-daemon` passes.
- [ ] `cargo test -p p2p-mobile` passes.
- [ ] Full `cargo test` passes (or unrelated pre-existing failures documented).
- [ ] `lintDebug`, `testDebugUnitTest`, `assembleDebug` pass.

### Secret-safety spot checks
```bash
# Status JSON / runtime status must not carry secret material.
grep -RnE "identity|private|password|token|secret" crates/p2p-daemon/src/status.rs
```
- [ ] No secret values are placed into `DaemonStatus`/`ForwardRuntimeStatus`/
      `last_error` (matches above are field plumbing/tests only, not leaked values).

### Manual QA (offer mode, physical device if possible)
- [ ] Start tunnel; Home shows real MQTT/connection state, not "task spawned".
- [ ] Each forward shows `Listening` only after its local port is actually bound.
- [ ] Misconfigure one forward's local port to force a bind error (per D1); confirm
      that forward shows `Error` while others show `Listening`.
- [ ] Stop tunnel; forwards transition to `Stopped`/cleared.
- [ ] Export diagnostics; confirm no private identity or secrets.

---

## Definition of done

- [ ] Daemon delivers `DaemonStatus` to `AndroidTunnelController` over a channel.
- [ ] Mobile status no longer fabricates `mqttConnected`/`activeSessionCount`/state.
- [ ] Per-forward `Listening`/`Stopped`/`Error` sourced from real offer binds and
      surfaced to the UI; disabled forwards derive `Disabled` from config.
- [ ] Older native JSON without `forwards` still decodes on Kotlin.
- [ ] All Rust + Android tests, lint, and debug build pass.
- [ ] No secret material in status JSON, status file, logs, or UI.
- [ ] UIUX2 Phase A/B behavior preserved.
- [ ] CLI behavior unchanged except additive, backward-compatible status fields.
