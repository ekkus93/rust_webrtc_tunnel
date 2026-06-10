# Responses to ANDROID_UIUX2_FOLLOWUP Spec & TODO

This file collects questions, issues, and verification notes from reviewing
`ANDROID_UIUX2_FOLLOWUP_SPEC.md` and `ANDROID_UIUX2_FOLLOWUP_TODO.md` against the
actual code in the tree. No code has been written yet.

**Bottom line:** The spec is accurate — every significant claim was verified
against the current source. P1–P4 and P7 are well-scoped, low-risk, and ready to
implement. The one thing genuinely blocking implementation is the **direction for
P5** (per-forward runtime status), because the spec's central assumption about
what the native runtime can honestly report does not match how the runtime is
currently built. Details below.

---

## 1. Verification — claims that check out

All confirmed against the current tree:

- **P1 (Logs):** `LogsScreen` (screens.kt:549–668) uses a `LazyColumn`, but
  `EmptyStateCard("No logs available.")` is rendered *outside* it (line 641),
  while the debug-hidden card *is* an item inside it. The spec's "partially
  satisfies H4" is exactly right.
- **P2 ("Android v1"):** Present in `TunnelForegroundService.kt:90` and `:200`.
  Note that `screens.kt:767` *already* says "not available on Android" — so the
  service string is simply out of sync with the settings screen. Trivial fix.
- **P3 (chip contrast):** `ForwardSummaryRow` takes `statusColor` and the
  `Surface` sets `color` but **no** `contentColor` (components.kt:101–105). Real
  contrast risk, confirmed.
- **P4 (localhost):** `testLocalPort` hardcodes `127.0.0.1`
  (AppViewModels.kt:672–676), and the **Home** "Open URL" button also hardcodes
  it (screens.kt:296) — but **ForwardDetails** already uses `forward.localHost`
  (screens.kt:473). The inconsistency is real and the spec correctly centralizes
  it. Config validation today permits only `127.0.0.1` or `localhost`
  (ConfigRepository.kt:252).
- **P5 model state:** `NativeRuntimeStatusDto` has **no** `forwards` field
  (Models.kt:77–84); `toTunnelStatus()` does **not** populate `TunnelStatus.forwards`
  — it `.copy()`s and preserves the previous list (TunnelRepository.kt:157–166).
  `ForwardStatus` and `ListenState` exist as the spec describes (Models.kt:28, 41–50).
  On the Rust side, `AndroidRuntimeStatus` has **no** `forwards` field and there
  is **no** per-forward status type (runtime.rs:31–39).
- **P6:** `TunnelRepository.refreshStatus()` exists (TunnelRepository.kt:48) and
  the native `status()` is a cheap mutex clone (runtime.rs:278). The service is
  currently event-driven (network monitor only); there is no status poll job.
- **P7:** All listed prior fixes are present in the tree.

---

## 2. BLOCKING — P5 honesty problem (need a decision)

The spec's own caveat ("do not fabricate granular per-forward success if the
native layer cannot know it") is the crux, and the code makes it sharper than the
spec lets on. Two facts about `AndroidTunnelController` (runtime.rs):

1. **The runtime has zero per-forward visibility.** It spawns
   `run_offer_daemon` / `run_answer_daemon` as one opaque task
   (runtime.rs:197–234) and only tracks daemon lifecycle
   (`Stopped/Starting/Running/Stopping/Error`). To report *true* per-forward
   listen state, you would need a status channel from the daemon back into the
   controller — substantially bigger than "add a `forwards` field."

2. **Even the daemon-level state is optimistic.** `state = Running` is set
   *synchronously the instant the task is spawned* (runtime.rs:236), before MQTT
   connects, before the peer connects, before any local listener binds. The
   Kotlin side already inherits this optimism: `toTunnelStatus()` sets
   `mqttConnected = active` and `activeSessionCount = 1` purely from that
   spawn-time `active` flag (TunnelRepository.kt:160–161) — neither value is
   actually measured.

Consequence: the "honest fallback" version of P5 (stamp all enabled forwards with
the daemon-level state) would flip **every** forward to **"Listening" the moment
Start is tapped**, regardless of whether anything is connected. A green
"Listening" chip is a *stronger* claim than the current "Configured" fallback, so
the coarse version is arguably *more* misleading than what's there today, not
less.

**Question (need to pick a direction):**

- **(a) Defer P5** until the daemon can report real per-forward / connection
  state. Keep the "Configured" fallback for now.
- **(b) Do the coarse version anyway**, accepting that chips track "tunnel
  started" rather than "actually listening/connected."
- **(c) Treat this as the trigger to add a real daemon→controller status
  channel** — largest scope, but it would also fix the already-fake
  `mqttConnected` / `activeSessionCount` values, not just per-forward state.

Our lean is **(a) or (c)**, not (b). If (b) is chosen anyway, the chip label
wording should be softened so it does not assert "Listening" for a connection
state the runtime cannot verify.

---

## 3. P6 — implementable, with one load-bearing guard

Polling is cheap and worthwhile *independently of P5*: when the spawned daemon
task eventually fails, the runtime flips to `Error` (runtime.rs:218–227), and
today the UI never sees that without a manual refresh. Polling fixes that.

**Trap to flag:** `refreshStatus()` does `previous.copy(serviceState = ...)` from
native state. When the tunnel is paused by network policy
(`PausedMeteredBlocked`, set in `setPolicyBlocked`, TunnelRepository.kt:101–113),
native still reports `active/running`, so a poll would **clobber the paused state
back to Connected**. The spec already says "polling must stop when paused by
policy" — we just want to flag that this guard is load-bearing, not optional. A
poll that races a policy pause must not resurrect the running state.

**Minor question:** while paused-by-policy, should polling fully stop, or continue
but have `refreshStatus()` learn to *not* override a policy-pause state? Stopping
is simpler and is what the spec implies; confirm that's the intent.

---

## 4. Minor notes (FYI, no decision needed)

- **P4:** `browserHostForLocalForward` handles `""` / `0.0.0.0` / `::` →
  `127.0.0.1`, but config validation permits none of those today, so those
  branches are pure future-proofing (dead until validation changes). Fine to
  keep — just noting they aren't exercised yet.
- **P5:** The proposed Kotlin DTO uses snake_case field names (`local_host`,
  etc.), which matches the existing `NativeLogEventDto` convention — no
  kotlinx-serialization naming surprise.
- **P2:** Recommend aligning the service string to the existing screen wording
  ("Answer mode is not available on Android") so both sites match exactly.

---

## 5. Scope & phasing question

P1–P4 + P7 are small, Kotlin-only, and zero-risk; they could land as one batch
immediately. P5 / P6 touch Rust + the native bridge and carry the honesty
question above. Given the previous two passes were done as phased commits, we
propose:

- **Phase A (now):** P1, P2, P3, P4, P7 — UI-only correctness/polish.
- **Phase B (after the P5 direction is settled):** P5 + P6 — native status
  bridge and polling.

Confirm whether you want this phased the same way, or held for a single PR.

---

## Summary of what we need back

1. **P5 direction:** (a) defer, (b) coarse, or (c) real daemon→controller status
   channel.
2. **P6 pause behavior:** stop polling while paused-by-policy (simple), or keep
   polling with a policy-aware `refreshStatus()`?
3. **Phasing:** phased (A then B) or single PR?

Everything else can proceed once these are confirmed.
