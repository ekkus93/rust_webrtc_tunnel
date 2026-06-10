# P5 — Real Per-Forward Runtime Status Specification

## Context

This spec covers **P5**, deferred from `ANDROID_UIUX2_FOLLOWUP_SPEC.md` (see the
decision in `replies1.md`). The UIUX2 follow-up (Phases A and B, commits `595b00f`
and `0e2fe12`) deliberately did **not** implement per-forward runtime status,
because the native runtime cannot currently report it honestly.

The problem, confirmed against the code:

1. `AndroidTunnelController` (`crates/p2p-mobile/src/runtime.rs:31-39`, `146-253`)
   only tracks daemon lifecycle (`Stopped/Starting/Running/Stopping/Error`), set
   **optimistically the instant the daemon task is spawned**
   (`runtime.rs:236`), before MQTT connects, before a peer connects, before any
   local listener binds.
2. There is **no status channel** from the daemon back to the controller — the
   controller only sees the final `Result<(), DaemonError>` when the task ends.
3. The Kotlin layer inherits this optimism: `TunnelRepository.toTunnelStatus()`
   sets `mqttConnected = active` and `activeSessionCount = if (active) 1 else 0`
   from the spawn-time `active` flag (`TunnelRepository.kt:160-161`). **Neither is
   measured.**
4. `TunnelStatus.forwards` is never populated from native, so the UI falls back to
   a `Configured` / `Disabled` label derived purely from saved config
   (`screens.kt` Home and Forwards rows).

Crucially, **a daemon-level status structure already exists** and is used by the
desktop CLI but is never wired to mobile:

- `DaemonStatus` (`crates/p2p-daemon/src/status.rs:9-20`) already aggregates
  `peer_id`, `role`, `mqtt_connected`, `active_session_id`, `current_state`
  (`DaemonState`), `active_session_count`, `session_capacity`, `sessions`, and
  `configured_forwards`.
- It is produced at a small number of choke points: `write_daemon_status`
  (`lib.rs:2052`), `write_answer_status` (`lib.rs:2067`), and
  `write_steady_state_status` (`lib.rs:2095`), all routed through
  `write_status_or_log(ctx.status, DaemonStatus)`.
- `StatusWriter::write()` (`status.rs:115`) serializes it to the CLI status file.

What does **not** exist anywhere: a per-forward runtime state structure
(`forward_id -> {listening | error | stopped}`). `DaemonStatus.configured_forwards`
and `SessionStatus.configured_forward_ids` are just lists of configured IDs, not
state. The status tests explicitly assert per-forward fields are absent
(`status.rs:159-160`, `196`, `278-279`).

The offer daemon binds one local `TcpListener` per forward in
`bind_offer_listeners` (`lib.rs:1992-2011` → `OfferListener::bind`,
`crates/p2p-tunnel/src/offer.rs:14`). **Bind success is the honest source of a
per-forward `Listening` state.** Today a bind failure is fatal for the whole
daemon, so there is no per-forward `Error` state.

## Goals

- Wire a real status channel from the daemon to `AndroidTunnelController`, reusing
  the existing `DaemonStatus` rather than inventing a parallel mechanism.
- Replace fabricated mobile status fields (`mqttConnected`, `activeSessionCount`,
  optimistic `state`) with measured values.
- Add honest per-forward runtime state for the **offer** role, sourced from real
  local-listener bind results, and surface it to the Android UI via the
  `NativeRuntimeForwardStatusDto` / `ForwardStatus` path already designed in the
  UIUX2 spec.
- Preserve the desktop CLI behavior except for additive, backward-compatible
  status-file fields.
- Preserve the security model: no private identity, broker credentials, tokens,
  certificate bodies, or unredacted secret paths in `DaemonStatus`,
  `AndroidRuntimeStatus`, the status file, logs, or UI.

## Non-goals

- Do **not** implement Android answer mode (it remains disabled).
- Do **not** fabricate per-forward "connected / data-flowing" state. Connection
  liveness is tracked per **session**, not per **forward** (`ActiveSession.state`,
  `data_channel_open`). Per-forward "Listening" (local bind) is the honest signal
  this spec delivers. Mapping accepted clients to a per-forward active-connection
  count is an explicitly optional enhancement (see §3.4).
- Do **not** redesign the daemon's session/reconnect state machine
  (`DaemonState`, `crates/p2p-core/src/protocol.rs:14-27`).
- Do **not** change network-policy defaults or the UIUX2 Phase A/B behavior.

## Affected files

Rust:
- `crates/p2p-daemon/src/status.rs` — extend `DaemonStatus`; add forward status type.
- `crates/p2p-daemon/src/lib.rs` — thread per-forward state; add status sink; new
  `*_with_status` entry points; populate forward state at bind time.
- `crates/p2p-mobile/src/runtime.rs` — receive `DaemonStatus` via channel; derive
  `AndroidRuntimeStatus` (incl. `forwards`) from it.
- `crates/p2p-mobile/src/lib.rs` — surface any new fields in the status JSON.
- CLI crate that calls `run_offer_daemon`/`run_answer_daemon` (delegation only).

Kotlin:
- `android/app/src/main/java/com/phillipchin/webrtctunnel/model/Models.kt`
- `android/app/src/main/java/com/phillipchin/webrtctunnel/data/TunnelRepository.kt`
- `android/app/src/main/java/com/phillipchin/webrtctunnel/ui/screens.kt` (fallback
  cleanup only)

Tests: existing Rust tests in `p2p-daemon`/`p2p-mobile`; Android unit tests under
`android/app/src/test/...`.

## Architecture decision

**Reuse `DaemonStatus`; deliver it over a `tokio::sync::watch` channel.**

`watch` is the right primitive: it is latest-value (mobile only cares about the
current state), cheap, lossless for "most recent", and pairs naturally with the
existing `refreshStatus()` polling added in UIUX2 Phase B. The daemon already
produces a fresh `DaemonStatus` at every meaningful transition; we send a clone of
each to the channel in addition to writing the file.

Rejected alternatives: a bespoke per-forward mpsc (more code, ordering concerns, no
benefit over latest-value); reading the CLI status **file** on mobile (path/lifecycle
coupling, redaction ambiguity, slower).

## Detailed requirements

### Phase 1 — Daemon → controller status channel (no schema change)

1. Define a status sink the daemon can push each `DaemonStatus` to. Recommended:

   ```rust
   pub type DaemonStatusSink = tokio::sync::watch::Sender<DaemonStatus>;
   ```

   Thread an `Option<&DaemonStatusSink>` (or a small `StatusEmitter` wrapping the
   `StatusWriter` + optional sink) through `RuntimeContext` (`lib.rs:256-260`). At
   the single `write_status_or_log` choke point, after writing the file, also
   `let _ = sink.send(status.clone());` when a sink is present. This guarantees the
   channel sees exactly what the file sees — one source of truth.

2. Add entry points that accept the sink without breaking the CLI:

   ```rust
   pub async fn run_offer_daemon_with_status(
       config: AppConfig, identity: IdentityFile, keys: AuthorizedKeys,
       status_sink: DaemonStatusSink,
   ) -> Result<(), DaemonError>;
   // existing run_offer_daemon delegates with no sink; CLI unchanged.
   ```

   Same for the answer daemon.

3. In `AndroidTunnelController::start` (`runtime.rs:146`):
   - Create `let (tx, rx) = tokio::sync::watch::channel(DaemonStatus::default-ish)`.
     (`DaemonStatus` currently has no `Default`; either add a derive or seed the
     channel with an explicit "starting" status built from the loaded config.)
   - Store `rx` in `RuntimeInner`.
   - Pass `tx` into `run_offer_daemon_with_status`.
   - Keep the existing optimistic `state = Starting/Running` only as the seed value
     until the first real snapshot arrives.

4. Rewrite `AndroidTunnelController::status()` (`runtime.rs:278`) to derive
   `AndroidRuntimeStatus` from the latest `DaemonStatus` in `rx.borrow()`, merged
   with controller-owned lifecycle facts (e.g. `config_path`,
   `started_at_unix_ms`, terminal `Error` set on task completion).

5. Map `DaemonState` (`protocol.rs:14-27`) → the mobile `state`/`active` honestly:
   - `Idle` / `WaitingForLocalClient` / `Serving` → running, listening for use.
   - `Negotiating` / `ConnectingDataChannel` → connecting.
   - `TunnelOpen` → connected.
   - `IceRestarting` / `Renegotiating` / `Backoff` → reconnecting.
   - `Closed` → stopped.
   The exact mapping to `AndroidRuntimeState` / Kotlin `ServiceState` must be
   documented in code and covered by tests.

6. Replace fabricated Kotlin fields with real ones now available:
   - `mqttConnected` ← `DaemonStatus.mqtt_connected`.
   - `activeSessionCount` ← `DaemonStatus.active_session_count`.
   - `sessionCapacity` ← `DaemonStatus.session_capacity`.

**Acceptance:** With Phase 1 only, Home reflects real MQTT/session/connection state
within the existing poll interval; `mqttConnected` is no longer just "task spawned."
No per-forward state yet.

### Phase 2 — Per-forward runtime state in the daemon (offer role)

1. Add to `status.rs`:

   ```rust
   #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
   #[serde(rename_all = "snake_case")]
   pub enum ForwardListenState {
       Listening,
       #[default]
       Stopped,
       Error,
   }

   #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
   pub struct ForwardRuntimeStatus {
       pub id: String,
       pub listen_state: ForwardListenState,
       pub last_error: Option<String>,
   }
   ```

2. Add `pub forwards: Vec<ForwardRuntimeStatus>` to `DaemonStatus`. This is
   additive to the CLI status file (acceptable; update the schema tests rather than
   weaken them — see §5).

3. Maintain a `forward_id -> ForwardRuntimeStatus` map in `DaemonRuntimeState`,
   threaded through `RuntimeContext`, and include it whenever a `DaemonStatus` is
   built (`write_daemon_status` / `write_answer_status`, `lib.rs:2052-2081`).

4. Populate it at `bind_offer_listeners` (`lib.rs:1992-2011`):
   - On successful `OfferListener::bind` → `ForwardListenState::Listening`.
   - On stop / steady-state teardown → `Stopped`.
   - On bind failure → `Error` with a redacted message. **Decision required**
     (see §6.1): this requires changing bind failure from fatal-for-the-daemon to
     soft-fail-per-forward. Without that change, a single bad forward still aborts
     startup and no `Error` chip is possible.

5. **Answer role:** answer-side forwards have no local listener (they dial
   `target_host:target_port` per session). Report answer forwards as `Stopped`
   (or omit) for now. Android is offer-only (answer mode disabled), so this is out
   of the active path; document the limitation explicitly.

**Honesty rule:** `Listening` MUST mean the local TCP listener is actually bound.
Never derive `Listening` from "the daemon task started."

### Phase 3 — Surface per-forward status to Android

This is the Kotlin work originally outlined in `ANDROID_UIUX2_FOLLOWUP_SPEC.md` §5,
now feasible because real data exists.

1. `crates/p2p-mobile`: extend `AndroidRuntimeStatus` with
   `forwards: Vec<AndroidForwardRuntimeStatus>` derived from
   `DaemonStatus.forwards`, using snake_case JSON to match Kotlin DTOs. Include the
   config-derived `name`, `local_host`, `local_port`, `remote_forward_id`,
   `enabled` so the Kotlin `ForwardStatus` can be built, plus `listen_state` and
   `last_error` from runtime.

2. Kotlin `Models.kt`:

   ```kotlin
   @Serializable
   data class NativeRuntimeForwardStatusDto(
       val id: String,
       val name: String,
       val local_host: String,
       val local_port: Int,
       val remote_forward_id: String,
       val enabled: Boolean = true,
       val listen_state: String,
       val last_error: String? = null,
   )
   ```
   and `val forwards: List<NativeRuntimeForwardStatusDto> = emptyList()` on
   `NativeRuntimeStatusDto` (defaulted for backward compatibility).

3. `TunnelRepository.toTunnelStatus()` maps native forward DTOs into
   `TunnelStatus.forwards` (`ForwardStatus`), with a **tolerant** listen-state
   mapper: `listening`/`stopped`/`error`/`disabled`/`paused` → enum, unknown →
   safe fallback (`Stopped`, or `Error` if `last_error` is present). Redact
   `last_error` via `SensitiveDataRedactor` before storing.

4. UI: keep the `Configured`/`Disabled` label only as a fallback for forwards with
   **no** runtime entry. When a runtime entry exists, render its real state through
   `forwardStatusChipColors()` (added in UIUX2 Phase A). Combine with the existing
   policy-pause safeguard from Phase B (a poll must not resurrect a paused state).

**Acceptance:** With the tunnel running, Home/Forwards show real
`Listening`/`Error`/`Stopped` per forward; disabled forwards still show `Disabled`
from config; older native JSON without `forwards` still decodes.

### Phase 4 — Tests & validation

See "Testing requirements."

## Security requirements

- `DaemonStatus.forwards`, `AndroidForwardRuntimeStatus`, and `last_error` must
  contain no secrets. `last_error` for a bind failure should be a generic reason
  (e.g. "address in use", "permission denied") — never include credentials, full
  filesystem paths to secret material, or identity bytes.
- The new status-file field is subject to the same redaction guarantees as the
  existing file; extend, do not weaken, the "no secrets" status tests.
- Kotlin must continue to route every native string through
  `SensitiveDataRedactor` before it reaches UI state.

## Testing requirements

### Rust (`p2p-daemon`, `p2p-mobile`)
- `DaemonStatus` serializes with `forwards` as an array; empty by default.
- Update the schema tests in `status.rs` (`current_status_schema_exposes_only_stable_public_fields`,
  the `open_forward_ids` assertions) to include `forwards` without removing the
  secret-absence checks.
- A successful offer bind yields `ForwardListenState::Listening` for that forward;
  a simulated bind failure yields `Error` with a non-secret message (depends on
  §6.1 decision).
- The status sink receives a `DaemonStatus` clone matching what is written to file.
- `AndroidTunnelController::status()` reflects channel values: fresh runtime →
  empty `forwards`; running runtime → forwards present; secret-safe.
- No secret strings appear in any serialized status.

### Kotlin
- Decode native status JSON **with** `forwards` → `TunnelStatus.forwards` populated.
- Decode native status JSON **without** `forwards` → `forwards == emptyList()`.
- Unknown `listen_state` string does not crash; maps to the documented fallback.
- `mqttConnected`/`activeSessionCount`/`sessionCapacity` reflect decoded values,
  not a hardcoded `active` flag.
- Policy-pause safeguard (Phase B) still holds when forwards are present.

### Validation commands
```bash
cargo test -p p2p-daemon
cargo test -p p2p-mobile
cargo test
cd android
./gradlew --no-daemon lintDebug
./gradlew --no-daemon testDebugUnitTest
./gradlew --no-daemon assembleDebug
```

## Open decisions (resolve before/while implementing)

1. **Per-forward bind: soft-fail vs. fatal.** To show a per-forward `Error` chip,
   `bind_offer_listeners` must soft-fail an individual forward (record `Error`,
   keep others listening) instead of aborting the whole daemon. This changes
   behavior shared with the desktop CLI. **Recommendation:** soft-fail per forward,
   but keep "zero forwards could bind" as a daemon-level error. Needs sign-off
   because it alters CLI startup semantics.
2. **Status delivery primitive.** `watch` (recommended) vs. callback trait vs.
   broadcast. Confirm `watch`.
3. **Per-forward active-connection count (optional).** The offer accept loop knows
   the `forward_id` of each accepted client (`run_offer_session`). We *could* track
   an active-connection count per forward. **Recommendation:** defer; `Listening`
   is sufficient for v1.
4. **Answer-role semantics.** Confirmed out of scope while answer mode is disabled.
5. **`DaemonStatus` default/seed.** Either derive `Default` for `DaemonStatus`
   (requires defaults for `PeerId`/`NodeRole`/`DaemonState`) or seed the watch
   channel with an explicit config-derived "starting" status. **Recommendation:**
   seed explicitly to avoid forcing questionable `Default`s on core types.

## Phasing recommendation

Phase 1 alone is a meaningful, shippable honesty win (real MQTT/session/connection
state) with no schema change. Phase 2+3 add per-forward state. Suggested commits:

- **Commit 1:** Phase 1 (status channel + honest connection/session fields).
- **Commit 2:** Phase 2 (daemon per-forward state, incl. the §6.1 decision).
- **Commit 3:** Phase 3 (Android surfacing) + Phase 4 tests.

## Definition of done

- Daemon delivers `DaemonStatus` to `AndroidTunnelController` over a channel.
- Mobile status no longer fabricates `mqttConnected`/`activeSessionCount`/state.
- Per-forward `Listening`/`Stopped`/`Error` is sourced from real offer binds and
  surfaced to the Android UI; disabled forwards still derive `Disabled` from config.
- Older native status JSON without `forwards` still decodes on Kotlin.
- All Rust and Android tests pass; lint and debug build pass.
- No secret material appears in status JSON, the status file, logs, or UI.
- UIUX2 Phase A/B behavior is preserved.
