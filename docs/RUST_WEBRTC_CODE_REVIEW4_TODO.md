# Rust WebRTC Tunnel Code Review 4 TODO

## Goal

Address the remaining issues from the latest review, with emphasis on:

1. accurate MQTT connectivity/status reporting
2. replay/dedup hardening for active busy-offer handling
3. clearer fatal-vs-recoverable runtime error policy
4. stronger daemon lifecycle/integration test coverage

This TODO is written to be explicit and implementation-oriented for GitHub Copilot.

---

## P0 — Fix inaccurate `mqtt_connected` status reporting

### Task P0.1 — Identify all status write sites

- Find every place where `DaemonStatus` is constructed or written.
- Enumerate where `mqtt_connected` is currently set.
- Document which of those writes happen:
  - at startup
  - during idle steady state
  - during session startup
  - during session teardown
  - during transport failure/recovery

### Task P0.2 — Add explicit daemon MQTT connectivity state

Implement explicit runtime state for MQTT connectivity instead of inferring it ad hoc.

Requirements:
- Add a small connectivity-state variable/field owned by the daemon runtime.
- It must represent real current transport state, not a guessed value.
- It must be updateable from:
  - successful connection/setup
  - transport poll failure/disconnect
  - successful reconnect/recovery
  - permanent fatal shutdown paths

### Task P0.3 — Use real connectivity state in `DaemonStatus`

- Stop hardcoding or defaulting `mqtt_connected = true` in status writes.
- Make all status writes use the tracked runtime connectivity state.
- Ensure session-state updates do not accidentally overwrite connectivity state with a stale optimistic value.

### Task P0.4 — Update status on recoverable transport errors

On recoverable MQTT/signaling transport errors:
- mark `mqtt_connected = false`
- write updated status if possible
- log the transition
- then proceed with recovery/backoff behavior

On successful recovery/reconnect:
- mark `mqtt_connected = true`
- write updated status
- log recovery

### Task P0.5 — Add tests for status accuracy

Add tests that validate:
- startup/healthy state writes `mqtt_connected = true`
- recoverable transport failure updates status to `false`
- recovery updates status back to `true`
- status-write failures remain recoverable and do not kill the daemon

---

## P1 — Harden active busy-offer replay/dedup behavior

### Task P1.1 — Locate the active busy-offer classification path

- Identify the function(s) responsible for classifying and responding to incoming offers during an already-active answer session.
- Confirm where replay/dedup data is currently sourced.
- Confirm whether a fresh replay cache/dedupe structure is created per call.

### Task P1.2 — Add persistent dedupe state for active busy offers

Implement a small persistent dedupe cache for this path.

Requirements:
- Scope may be per active answer session or per daemon, but must persist across repeated calls while the session remains active.
- Key should include enough information to suppress duplicate handling, such as:
  - sender KID
  - message ID
  - optionally session ID if appropriate
- The cache must only suppress exact duplicates/replays, not legitimate new offers from distinct peers/messages.

### Task P1.3 — Suppress repeated `busy` responses for duplicates

Behavior to enforce:
- first allowed peer offer during active session may receive encrypted `busy`
- repeated duplicate/replayed copies of that same offer must **not** produce repeated `busy` replies
- unauthorized or disallowed peers must continue to receive **no response**

### Task P1.4 — Add tests for busy-offer dedupe

Add tests that prove:
- allowed peer receives one `busy` for a legitimate first foreign offer during active session
- exact duplicate/replayed copies do not trigger additional `busy` replies
- unauthorized peer receives no response
- disallowed-but-authorized peer receives no response

---

## P2 — Freeze and implement fatal-vs-recoverable runtime policy

### Task P2.1 — Enumerate current fatal error paths

Audit the daemon code and identify where errors can bubble out and terminate the process.

Create a list of current fatal paths for:
- startup/config failures
- identity/authorized-key loading failures
- transport setup failures
- runtime transport turbulence
- accept-loop failures
- session failures
- status write failures

### Task P2.2 — Classify errors into fatal vs recoverable

Freeze and implement the following policy.

#### Fatal
These should terminate the daemon:
- invalid config
- invalid/missing identity files
- invalid/missing authorized keys
- TLS/security misconfiguration
- cryptographic initialization failure
- startup bind failure that prevents entering service
- other startup/init failures that prevent the daemon from functioning at all

#### Recoverable
These should not kill the daemon:
- individual session failures
- ICE failure for one session
- ACK timeout for one session
- target-connect failure for one session
- remote error/close for one session
- transient signaling transport poll/read errors
- transient signaling publish failures
- local status file write failures
- ordinary accept-loop turbulence if service can continue

### Task P2.3 — Wrap recoverable runtime failures consistently

- Replace remaining top-level `?` propagation for recoverable runtime conditions with explicit recovery handling.
- Ensure recoverable paths:
  - log the error
  - clean up any current session state
  - update status if relevant
  - optionally back off
  - return to idle/waiting state

### Task P2.4 — Keep fatal paths explicit and obvious

Do **not** silently recover from:
- broken identity/security setup
- invalid config
- impossible startup conditions

Fatal startup/security failures should still fail fast and loudly.

### Task P2.5 — Add tests for recoverable runtime behavior

Add tests that validate:
- session failure does not kill daemon
- recoverable signaling transport failure does not kill daemon
- status write failure does not kill daemon
- daemon returns to steady state after cleanup

---

## P3 — Add higher-level lifecycle/integration tests

### Task P3.1 — Add top-level daemon behavior tests

Add tests around top-level daemon orchestration, not just helper components.

Target scenarios:
- answer daemon survives a failed session and returns to waiting
- offer daemon survives a failed session and returns to waiting for the next local client
- active offer-side session rejects extra local clients while busy

### Task P3.2 — Add status transition tests

Add tests for:
- healthy startup status
- disconnect status
- reconnect status
- session active/inactive transitions
- status write failure remains recoverable

### Task P3.3 — Add busy-offer policy tests

Add higher-level tests covering:
- active answer session + allowed peer foreign offer => one `busy`
- replayed duplicate => no repeated `busy`
- unauthorized peer => no response
- authorized-but-disallowed peer => no response

### Task P3.4 — Add runtime turbulence tests

Add tests for:
- transient signaling transport poll failure
- transient signaling publish failure
- cleanup then return to steady state

These do not need to be full network integration tests; controlled fakes/mocks for transport are acceptable if they truly exercise top-level orchestration.

---

## P4 — General cleanup and documentation alignment

### Task P4.1 — Audit remaining config/runtime alignment

- Re-check all public config fields.
- For each field, confirm one of the following is true:
  - it meaningfully affects runtime behavior, or
  - it is explicitly unsupported and rejected, or
  - it should be removed

### Task P4.2 — Update docs/spec if runtime policy changed

If the implementation clarifies or changes any runtime behavior, update the docs/spec accordingly.

Specifically ensure the docs match actual behavior for:
- daemon recoverability
- busy local client handling
- active busy-offer response policy
- status semantics

### Task P4.3 — Improve log messages around recovery

Ensure logs clearly distinguish:
- fatal startup/security failure
- recoverable runtime failure
- session failure with daemon survival
- transport disconnect/recovery
- status write failure

This will make debugging much easier.

---

## Suggested implementation order

1. **P0 — Fix `mqtt_connected` status reporting**
2. **P1 — Harden active busy-offer dedupe/replay behavior**
3. **P2 — Freeze and enforce fatal vs recoverable runtime policy**
4. **P3 — Add higher-level lifecycle/integration tests**
5. **P4 — Final cleanup/docs/logging pass**

---

## Expected outcome

After this TODO is completed, the codebase should have:

- more trustworthy local health/status reporting
- cleaner behavior for duplicate/replayed busy offers
- clearer and more robust daemon survival semantics
- better test coverage for the actual remaining runtime risk areas

That would make the project materially closer to production-readiness.
