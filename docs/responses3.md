# Responses to WEBRTC_TUNNEL_HARDENING SPEC + TODO — Questions & Issues

Review of `docs/WEBRTC_TUNNEL_HARDENING_SPEC.md` and
`docs/WEBRTC_TUNNEL_HARDENING_TODO.md`, cross-checked against the current source. No
code written.

**Overall:** Well-founded pass. Every high-risk pattern the review names actually exists
in the tree (verified below). The theme — "a failed primary path must not be disguised as
success/empty/default/another transport" — is right and matches where this codebase has
historically hidden failures. The notes below are (a) places where the spec's mental model
has drifted from what we already shipped, (b) coupling between tasks the TODO treats as
independent, and (c) a few items that are already done. Four questions are genuinely
blocking; the rest are clarifications.

---

## 0. Verified code facts (the review is accurate)

- **8.8.8.8 local-IP probe** — real, two sites: `crates/p2p-mobile/src/diagnostics.rs:91`
  and `crates/p2p-webrtc/src/lib.rs:572` (`primary_local_ipv4`).
- **`{}` / `[]` JNI decode fallbacks** — real, three sites in
  `crates/p2p-mobile/src/jni_bridge.rs`: line 130 (`"{}"`), line 148 (`"[]"`), line 350
  (`"{}"`).
- **`FakeTunnelBridge` in production source** — real, lives in
  `android/app/src/main/java/com/phillipchin/webrtctunnel/RustTunnelBridge.kt` (test at
  `src/test/.../FakeTunnelBridgeTest.kt`).
- **`recentLogs()` → `emptyList()`** — real, `TunnelRepository.kt:113`
  (`.getOrDefault(emptyList())`).
- **Unknown ICE state → `New`** — real, the `From<RTCIceConnectionState>` impl in
  `p2p-webrtc/src/lib.rs` ends `_ => Self::New`.
- **TURN rejection** — exists in `build_rtc_configuration` but the message is
  `"TURN URLs are not supported in v1"`, and it fires at WebRTC-construction time, not in
  config `validate.rs`.

---

## 1. BLOCKING — P0-001's premise is inverted vs. the code we shipped

The spec describes `auto` as "try vnet, then fall back to native ICE." The actual
`decide_ice_path` (`p2p-webrtc/src/lib.rs:413`) is the **opposite**:

- `auto` + interface enumeration **works** → **Native**
- `auto` + interface enumeration **fails** → **vnet_mux** (best-effort, `required: false`)

And per the comment at `lib.rs:408`, `android_ice_mode` is **honored on all platforms —
there is no Android-vs-desktop branch anywhere in the Rust code.**

Why this matters: the genuinely dangerous Android case is when enumeration **succeeds** →
`auto` picks **Native** → that is the exact black-hole path we proved broken on the A54.
The "auto → mux fold" added in the prior session only engages when enumeration **fails**.
So as written, `auto` on an Android device where enumeration works will still silently
choose the broken native path. "Make the Android default strict" therefore **cannot be
expressed in the Rust mode logic alone** — the code cannot tell it is running on Android.

**Q1 (blocking):** How should "Android-default-strict" be expressed?
- (a) `cfg(target_os = "android")` gating in Rust (so the Rust default differs per platform), or
- (b) the Kotlin layer picks an explicit non-`auto` mode as the Android default and leaves
  the Rust default alone.

This is the central decision; Q2 and Q3 depend on it.

---

## 2. BLOCKING — the recommended mode names drop the `mux` dimension, which is the fix

The spec recommends `vnet_required / native_required / auto_strict / auto_best_effort`.
But on Android the proven-working path is specifically **`vnet_mux`** (a single
`0.0.0.0`-bound UDP-mux socket, real interface IP advertised as the host candidate). Plain
**`vnet`** (socket pinned to the interface IP) is the *broken* one — it is the original
black-hole. So **`vnet_required` is ambiguous**: it does not say whether the mux is
engaged, and mux-vs-no-mux is exactly what decides whether offer→answer data flows.

Current modes in the config schema: `native`, `vnet`, `vnet_mux`, `auto`.

**Q2 (blocking):** Which way do you want this?
- (a) Keep the current 4 modes; just change the Android **default** to strict `vnet_mux`
  and add the fail-loud + status fields. Minimal schema churn.
- (b) Do the full rename to the spec's names. If so I need a mapping that preserves the mux
  distinction — e.g. `vnet_required` must mean "vnet **with** mux" (the working path), and
  we either drop or alias plain non-mux `vnet`. This touches `ConfigTemplates.kt`,
  `validate.rs`, the example TOMLs, the Android `VALID_ANDROID_ICE_MODES` set, and docs.

My recommendation: (a). It satisfies "default is strict, no silent native fallback"
without a schema migration, and directly closes the A54 hole.

---

## 3. BLOCKING — P0-002 and P0-001 are coupled (the TODO treats them as independent)

The 8.8.8.8 probe (`primary_local_ipv4`) feeds `fallback_net()`, which supplies **the IP
that vnet/vnet_mux advertises as its host candidate.** If we remove 8.8.8.8 on Android, the
vnet_mux path has **no address to advertise** unless Kotlin hands one in via JNI. So strict
vnet (P0-001) cannot fully land without the address-injection plumbing (P0-002) arriving
**together** — they are not separable in the order the TODO implies.

**Q3 (blocking):** What is the scope/sequencing for P0-002?
- (a) Full `ConnectivityManager` / `LinkProperties` query in Kotlin → new JNI parameter
  feeding the Rust config, done now (so strict vnet_mux has a real injected address); or
- (b) Phased: remove 8.8.8.8 from the Android path and **fail loud** if no address is
  injected, keep the desktop UDP-route probe as the non-Android fallback, and land the
  Kotlin `ConnectivityManager` query as the immediate follow-up.

---

## 4. BLOCKING (scope) — P0-005 is largely already implemented

`crates/p2p-tunnel/src/probe.rs` **already** performs a bidirectional `Ping`→`Pong` round
trip, and the offer does **not** start TCP forwarding until the matching `Pong` returns; we
also added the mid-session self-heal heartbeat last session. So the core of P0-005 is done.

What genuinely remains:
- the **answer-side `ProbingDataPlane` status state** (this is really P2-006),
- a **one-way-only failure** test (offer→answer delivered but no pong),
- **diagnostics wording** that only says `echo`/`round trip` when it happened (P2-005).

**Q4 (scope confirm):** Agreed that P0-005 collapses to "add the answer-side status state +
tighten wording + add the one-way test," rather than rebuilding the probe?

---

## 5. Non-blocking clarifications

- **TURN (4.1):** rejection exists but (i) the message is `"... not supported in v1"`, not
  the spec's suggested wording, and (ii) it fires at WebRTC-construction time, not in config
  `validate.rs`. Confirm you want it moved/duplicated into config validation with the
  spec's message — i.e. "fails before tunnel startup," not only at peer construction.
- **P1-007 status fields:** some already exist (heartbeat state landed last session). I will
  treat this as "add the missing fields" (requested ICE mode, selected path, fallback
  bool/reason, local address source, candidate counts), not a greenfield status struct.
  Flag if you expected otherwise.
- **P2-001 (Unknown ICE state):** straightforward — add an `Unknown`/`Unspecified` variant
  and stop mapping unmapped upstream states to `New`. No question, just confirming it is a
  small, self-contained change.

---

## 6. Effort read (for sequencing)

The only two genuinely large items are **P0-002** (Kotlin `ConnectivityManager` query +
JNI plumbing) and the **mode rename** *if* you pick (b) in Q2. Everything else
(P0-003/004, P1-001/002/003/004/005, P2-*) is mechanical and low-risk: error-message
propagation, replacing `{}`/`[]`/`emptyList()` fallbacks with explicit error objects,
moving the fake bridge to a test/debug source set, `ByteArray.fill(0)` in `finally`, and
regression tests asserting the no-silent-fallback rule.

**Suggested path to keep momentum (pending Q1/Q2):** keep the existing 4 mode names, make
the Android default `vnet_mux` (strict/required) — via `cfg(target_os = "android")` or the
Kotlin default — treat `auto` as the explicit best-effort opt-in, and land P0-002's
address-injection alongside it so strict vnet has an IP to advertise. That closes the real
A54 hole with no schema migration.
