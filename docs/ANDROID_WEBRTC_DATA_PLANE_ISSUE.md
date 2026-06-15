# Android WebRTC tunnel: data channel opens but SCTP user data never flows to a remote peer

**Status:** open. Root cause localized but not yet proven. This document is a
self-contained brief for deep research (assume no prior context).

---

## 1. Executive summary

We have a Rust WebRTC-based secure TCP tunnel (offer/answer over a multiplexed,
reliable/ordered WebRTC **data channel**; MQTT is the untrusted signaling relay; STUN
only, **no TURN**). It works fine desktop↔desktop and desktop↔server.

On **Android**, the offer side reaches a fully-connected WebRTC session — ICE
`connected`, DTLS established, SCTP association `Established`, and the DCEP data channel
**opens** (`DATA_CHANNEL_ACK` received) — but then **application data never completes the
round trip**. Specifically, **offer→answer SCTP DATA/SACK is not delivered** (the sender
hits `T3-rtx` retransmit timeouts; the peer retransmits the same TSN), while
**answer→offer receive works**. A browser / `nc` hitting the offer's local forward gets
**0 bytes**.

The failure is **narrow**: it only reproduces with a **specific physical Android phone
(Samsung A54, Android 16) talking to a specific remote answer (a Dockerized `p2p-answer`
at a coworking space, behind that network's NAT/firewall)**. The *same phone* works
against every *local* answer we can construct (same-LAN, Docker-bridge-NAT, even the phone
on cellular → a home answer). A **Linux laptop** offer works against the **same remote
answer** over the identical public candidate pair. So it is neither the tunnel code, nor
the remote answer, nor "NAT in general" — it is a **specific interaction between an
Android-only WebRTC networking fallback and the remote network's NAT/firewall**.

**Leading hypothesis:** Android 11+ restricts `getifaddrs`/NETLINK, so `webrtc-rs` gathers
no host candidate; our code works around that by injecting a fallback interface via
`SettingEngine::set_vnet(Net::Ifs(...))`. We suspect that in this vnet mode webrtc-rs
sends SCTP **data** from a different socket/source-port than the STUN connectivity checks
that established the NAT mapping. A **cone NAT** (home router, cellular carrier) reuses one
mapping regardless and tolerates this; a **symmetric / address-dependent NAT** (likely the
coworking firewall) drops the mismatched flow. The laptop's single native socket is
immune. **This is unproven** — it is the most consistent explanation, and the open
questions in §10 are aimed at confirming or refuting it.

---

## 2. System under test

- **Architecture:** multiple logical TCP streams multiplexed over **one** reliable,
  ordered WebRTC data channel per peer session. Offer binds local TCP listeners; on a
  local client connection it lazily negotiates a WebRTC session to the configured remote
  peer; the answer connects the stream to its local target and proxies bytes back.
- **Signaling:** MQTT over TLS (`mqtts://broker.emqx.io:8883`), end-to-end encrypted +
  signed (Ed25519 / X25519). Signaling is **proven healthy** in the failing case
  (HELLO / Offer / Answer / ICE candidates / Acks all flow; ICE reaches `connected`).
- **NAT traversal:** STUN only (`stun:stun.l.google.com:19302`). **No TURN** (the project
  deliberately rejects `turn:`/`turns:` URLs).
- **WebRTC stack (Rust `webrtc-rs`), exact versions:**
  - `webrtc` **0.8.0**
  - `webrtc-ice` **0.9.1**
  - `webrtc-sctp` **0.8.0**
  - `webrtc-dtls` **0.7.2**
  - `webrtc-data` **0.7.0**
  - `webrtc-util` **0.7.0**
  - `webrtc-mdns` **0.5.2**
- **Android app:** Kotlin + Jetpack Compose; a foreground service hosts the Rust runtime
  via JNI (`p2p-mobile` crate). Same Rust core as desktop.

---

## 3. The symptom (precise)

In the failing case, on a single request:

1. Offer accepts the local TCP client (`p2p_tunnel::offer: accepted local forward client`).
2. WebRTC negotiation completes:
   - `ice: ICE connection state changed (state=connected)`
   - `webrtc::peer_connection: peer connection state changed: connected` (DTLS done)
   - SCTP: `state change: 'Closed' => 'CookieWait'` → `sending INIT` → `chunkInitAck
     received` → `sending COOKIE-ECHO` → `'CookieEchoed' => 'Established'`
   - `webrtc_data::data_channel: Received DATA_CHANNEL_ACK` (the data channel is OPEN)
3. Then it **stalls**: `webrtc_sctp ... T3-rtx timed out: n_rtos=1 cwnd=1228 ssthresh=4912`
   — the offer's outbound DATA chunks are never SACKed. The peer keeps **retransmitting
   the same TSN** (`recving 92 bytes` ... `peer_last_tsn` stuck). The offer keeps
   receiving from the answer but its own sends do not land.
4. The local client receives **0 bytes**.

So: **answer→offer works; offer→answer does not, after the handshake.** STUN connectivity
checks on the selected pair keep succeeding for ~28 s (ICE stays `connected`), yet the
SCTP DATA does not traverse — small probe packets get through, sustained data does not.

---

## 4. The key result — failure matrix

Every row was actually run and observed.

| Offer phone / host | WebRTC fallback? | Answer location / mode | Result |
|---|---|---|---|
| **LG G6** (Android **8.0**, SDK 26, arm64) | **No** (pre-11, enumeration works) | local Docker, `--network host`, same LAN | ✅ bytes flow |
| **Samsung A54** (Android **16**) | **Yes** (`Net::Ifs` vnet) | local Docker, `--network host`, same LAN | ✅ bytes flow |
| Samsung A54 | Yes | local Docker, **bridge** (Docker NAT), same LAN | ✅ bytes flow |
| Samsung A54 | Yes | **A54 on T-Mobile cellular** → home Docker answer (two networks) | ✅ bytes flow |
| **Linux laptop** (`offer-arisu`) | No | **remote answer-office** (coworking, Dockerized) | ✅ bytes flow |
| **Samsung A54** | **Yes** | **remote answer-office** (coworking, Dockerized) | ❌ **0 bytes, SCTP T3-rtx** |

**Only the last row fails.** It is the unique intersection of (a) the Android vnet
fallback and (b) the coworking network's NAT/firewall.

---

## 5. The decisive laptop-vs-phone comparison (original remote setup)

Captured live, same Wi-Fi/NAT, same moment, same remote answer-office. Selected ICE
candidate pairs (from `webrtc_ice` TRACE, `agent_internal.rs:330` "Set selected candidate
pair"):

```
A54   (FAIL):  udp4 srflx 24.130.174.186:45766 related 0.0.0.0:45766
            <-> udp4 srflx 162.229.61.169:36114
laptop (OK):   udp4 srflx 24.130.174.186:48473 related 0.0.0.0:48473
            <-> udp4 srflx 162.229.61.169:36415
```

**Identical shape:** same local public IP `24.130.174.186` (phone and laptop share the
home NAT), same remote `162.229.61.169`, both `srflx↔srflx`, both `related 0.0.0.0` base.
Laptop's data flows (HTTP 200, 6917 bytes); A54's stalls (`T3-rtx`, 0 bytes). The
**candidate selection is the same** — the difference is in the actual data-plane behavior
of the two endpoints.

Other observed candidates in that session:
- A54 local: `host 192.168.88.106` (Wi-Fi, via the injected fallback), `srflx 24.130.174.186`.
- answer-office: `host 172.17.0.4` (Docker default bridge — unreachable from the phone,
  wastes checks), `srflx 162.229.61.169`.
- IPv6 srflx always fails benignly on the phone: `[controlled]: could not get server
  reflexive address udp6 stun:... Network is unreachable (os error 101)` — IPv4 srflx
  succeeds, ICE still connects.

---

## 6. The Android-specific code path (prime suspect)

`crates/p2p-webrtc/src/lib.rs`, `build_setting_engine()` (~line 396):

```rust
fn build_setting_engine() -> SettingEngine {
    let mut engine = SettingEngine::default();
    if os_interface_enumeration_works() { return engine; }   // desktop, LG G6 (Android 8)
    match fallback_net() {
        Some(net) => {
            engine.set_vnet(Some(Arc::new(net)));            // Android 11+ takes this path
            // log: "OS interface enumeration unavailable (e.g. Android NETLINK
            //       restriction); injecting a fallback host interface ..."
        }
        None => { /* warn: no fallback IP */ }
    }
    engine
}

fn os_interface_enumeration_works() -> bool {
    match ifaces::ifaces() {                                  // webrtc_util::ifaces
        Ok(list) => list.iter().any(|i| matches!(i.addr, Some(a) if a.is_ipv4() && !a.ip().is_loopback())),
        Err(_) => false,                                      // Android 11+: getifaddrs/NETLINK restricted
    }
}

fn fallback_net() -> Option<Net> {
    let ip = primary_local_ipv4()?;                          // see below
    let ipnet = IpNet::new(IpAddr::V4(ip), 24).ok()?;        // /24 placeholder; prefix irrelevant to gathering
    let interface = Interface::new("p2p-fallback".to_owned(), vec![ipnet]);
    Some(Net::Ifs(vec![interface]))                          // webrtc_util::vnet::net::Net
}

fn primary_local_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;                      // connect-less; no packets sent
    /* read socket.local_addr() -> the OS-chosen source IPv4 */
}
```

Why this exists: Android 11+ (API 30+) restricts `getifaddrs`/NETLINK for apps, so
`webrtc-rs` enumerates **no** interfaces and gathers **no host candidate** — it would only
offer a srflx candidate that a same-LAN answer cannot reach. The fallback injects the
OS-discovered LAN IP so a host candidate is gathered (commit "fix(android): gather a host
ICE candidate when OS enumeration is restricted").

**Crucial facts about `Net::Ifs`** (read from `webrtc-util-0.7.0/src/vnet/net.rs`):
- `Net::Ifs` is the **real-socket** vnet variant (not a fully virtual network):
  `Net::Ifs(_).bind(addr) => Ok(Arc::new(tokio::net::UdpSocket::bind(addr).await?))`.
- So it is essentially a **passthrough to real OS UDP sockets**; it only overrides the
  *interface list* reported to webrtc-rs (so host-candidate gathering has an IP to use).
- The selected-pair **DATA write** and the **STUN connectivity checks** both go through
  the same call: `webrtc-ice-0.9.1/src/candidate/candidate_base.rs:276`
  `conn.send_to(raw, addr).await`.

**Important dead-end we ruled out:** switching to a single muxed socket
(`UDPNetwork::Muxed`) is **not** a fix — `webrtc-ice-0.9.1/src/agent/agent_gather.rs:115`
does `UDPNetwork::Muxed(_) => continue` in the srflx-gathering loop, i.e. **Muxed sockets
do not gather server-reflexive candidates at all**, so a phone behind NAT could never
reach a remote peer. The `Ephemeral` (default) per-candidate-socket mode is the only one
that yields srflx — and it is the mode in use.

> **Open question this raises (see §10):** in `Ephemeral` + `Net::Ifs` mode, does
> webrtc-rs bind a **separate UDP socket per candidate** (host vs srflx), and does the
> srflx candidate's data egress from a **different socket/source-port** than the one whose
> mapping the answer learned? If yes, a symmetric NAT would drop it.

---

## 7. Everything we tried (chronological)

1. **"Self-targeting" red herring → fixed.** Early status showed the offer session's
   `remote_peer_id` equal to the *local* peer id, suggesting it dialed itself. This was a
   **display bug** in `crates/p2p-daemon/src/status.rs` (`DaemonStatus::new` stamped the
   local peer as the session remote). The real session targets the configured remote.
   Fixed end-to-end (daemon + Android UI); **not** the data-plane cause.

2. **Forward-id mismatch → ruled out.** The phone used forward id `llama` (wizard default)
   vs the laptop's `web-ui`. The answer rejects unknown ids
   (`p2p-tunnel/src/multiplex/answer.rs` `unknown_forward`). Changed the phone to the
   known-good `web-ui`, restarted — **still 0 bytes**. So not forward-id.

3. **Browser-specific → ruled out.** Reproduced with `toybox nc` (raw HTTP) on the phone:
   also **0 bytes**. Local TCP accept fires (offer logs `accepted local forward client`).
   Not a browser/cleartext/localhost issue.

4. **SCTP-level trace (the core evidence).** With a temporary debug build raising
   `webrtc_sctp`/`webrtc_ice` to DEBUG/TRACE and a file sink, captured the full sequence
   in §3: handshake OK, `DATA_CHANNEL_ACK` received, then **offer→answer T3-rtx**, answer
   retransmits the same TSN; answer→offer receive works.

5. **Selected candidate pair (§5).** Confirmed the A54 and laptop pick the **same**
   srflx↔srflx pair on the remote setup; laptop works, A54 doesn't.

6. **`Net::Ifs` source read.** Confirmed it is a real-socket passthrough and that DATA +
   STUN use the same `send_to` call (§6). So at the socket-API level the phone's path looks
   identical to the laptop's — pointing at either a per-candidate-socket subtlety or an
   Android/NAT-level effect rather than an obvious code bug.

7. **Muxed-socket idea → ruled out** (`agent_gather.rs:115`, no srflx; §6).

8. **Permanent redacted frame instrumentation added** (kept in-tree, debug level): offer
   `OPEN sent / OPEN-ack received / CLOSE/ERROR`; answer `OPEN received / target TCP
   connected`; the shared writer logs **data-channel send failures** (warn) — previously
   silent. (`crates/p2p-tunnel/src/multiplex/{offer,answer,stream}.rs`.)

9. **Wedged-session timeout added.** If the data channel never opens, the offer used to
   wait forever; now bounded (returns to listening). Note: this does **not** help the
   present failure, because here the channel *does* open and *then* data stalls.

10. **Controlled local rig (phone ↔ dockerized `p2p-answer`).** Built/repaired an e2e
    harness that drives the real Android app through its setup wizard and runs a
    Dockerized answer with full both-sides logging. Then ran the **failure matrix (§4)**:

    - A54 + local answer, `--network host`, same LAN → **works**.
    - A54 + local answer, **Docker bridge (NAT)**, same LAN → **works** (ICE connects via
      conntrack hole-punching; full SCTP + `answer received OPEN frame` + `answer target
      TCP connected`).
    - A54 on **T-Mobile cellular** → home Docker answer (genuinely two networks) →
      **works**. (Cellular here is IPv6-primary + 464XLAT/CLAT, `NOT_METERED`.)
    - LG G6 (Android 8, **no vnet**) + local answer → **works**.

    Conclusion: no local network shape reproduces it; only the real remote answer-office
    does.

---

## 8. What is ruled out (with evidence)

- **Tunnel / multiplex / answer application code** — every local path delivers bytes;
  answer-side frame logs show `received OPEN` + `target TCP connected`.
- **Signaling / broker / identity / authorization** — signaling completes; ICE connects;
  the answer accepts the session (would fail otherwise).
- **Forward-id mismatch** — tested with the known-good id (§7.2).
- **Browser behavior** — `nc` reproduces (§7.3).
- **ICE connectivity** — ICE reaches `connected`; STUN checks keep succeeding on the
  selected pair.
- **DTLS / SCTP handshake** — both complete; the data channel opens (`DATA_CHANNEL_ACK`).
- **The vnet fallback *alone*** — A54 (vnet) works local same-LAN, local Docker-NAT, and
  cellular→home.
- **Docker / NAT *in general*** — A54 works through Docker-bridge NAT and through
  carrier↔home double-NAT locally.
- **The phone's NAT being the culprit** — the laptop on the *same* home NAT works to the
  remote answer.
- **Candidate selection** — A54 and laptop select the same srflx pair.
- **IPv6 / mDNS** — IPv6 srflx fails benignly; mDNS init also fails on Android; neither
  blocks the (working) IPv4 path.
- **MTU on small packets** — the offer→answer payloads here are tiny (an HTTP GET ~80 B +
  SACKs); they still don't traverse.
- **The `related 0.0.0.0` srflx base** — the laptop has the same base and works.

---

## 9. Leading hypothesis (unproven) + alternatives

**Primary:** In `Ephemeral` + `Net::Ifs` (vnet) mode, webrtc-rs's selected-pair **data**
egresses from a different UDP socket / source 5-tuple than the **STUN** packets that
established the answer-side NAT mapping. A **cone NAT** (home, cellular) reuses one mapping
per internal endpoint → tolerant. A **symmetric / address-dependent NAT** (likely the
coworking firewall) creates per-destination mappings and **drops** packets arriving on an
unexpected mapping → the offer→answer data is silently dropped, the answer never SACKs, the
offer hits `T3-rtx`. The laptop's native single-socket path is immune.

**Alternatives worth investigating (for deep research):**
- The vnet `Net::Ifs` path subtly changes socket **binding/source-port** vs native
  (e.g. binds the data socket to a specific IP/ephemeral port that differs from the
  STUN-validated one), independent of the NAT-symmetry framing above.
- An interaction with **464XLAT/NAT64** or the coworking firewall's handling of the
  specific source — though note the *laptop* traverses the same coworking path fine.
- A **webrtc-rs 0.8.0 / webrtc-ice 0.9.1 bug** in vnet-mode `Ephemeral` socket reuse or
  candidate-pair selection nomination that only manifests against certain NATs; check
  whether newer webrtc-rs changed `Net::Ifs`/`Ephemeral`/socket handling.
- The coworking firewall doing **stateful UDP filtering / consent expiry / rate limiting**
  that the vnet path triggers but native doesn't (e.g. different DSCP, different packet
  cadence, ICE consent-freshness behavior in vnet mode).
- **Asymmetry direction**: confirm whether it's truly the offer's *send* not arriving, vs
  the answer's *SACKs* not arriving (both produce T3-rtx-looking symptoms). Needs captures
  on **both** ends.

---

## 10. Open questions for deep research

1. In `webrtc-rs` 0.8.0 (`webrtc-ice` 0.9.1) **Ephemeral** UDP network with a `Net::Ifs`
   vnet injected via `SettingEngine::set_vnet`, **which socket sends the selected-pair
   DTLS/SCTP data**, and is it the **same** socket (same source port) that gathered the
   srflx candidate and runs STUN consent checks? If different → symmetric NATs break it.
2. Does `Net::Ifs` cause sockets to **bind to a specific IP** (the injected interface IP)
   rather than `0.0.0.0`, and could that change the NAT mapping vs native?
3. Is there a known `webrtc-rs` issue about **vnet / `set_vnet` / `Net::Ifs` breaking
   media/data through symmetric NATs** while connectivity checks pass?
4. What is the **correct way to gather a host (and srflx) candidate on Android 11+** given
   `getifaddrs` is restricted, **without** the `set_vnet` workaround — e.g. a custom
   `UDPMux`/socket that still gathers srflx, or feeding webrtc-rs a pre-bound socket?
   (Note our finding: `UDPNetwork::Muxed` skips srflx.)
5. Would **`SettingEngine::set_nat_1to1_ips`** (Host type, which is STUN-compatible) be a
   cleaner host-candidate injection than `set_vnet`, and does it keep srflx + a single
   consistent socket?
6. Is this ultimately a case where **TURN is the only robust fix**, and if so, why does
   the *laptop* succeed on the same remote NAT without TURN (i.e. what exactly does the
   Android vnet path do differently that TURN would paper over)?

---

## 11. How to reproduce / tooling (in-repo)

- **Original failing path:** Android A54 (home Wi-Fi) offer → `answer-office` (remote,
  coworking, Dockerized). Currently the only repro. (answer-office is intermittently up.)
- **Controlled rig (works, does NOT reproduce, but great for iteration + both-sides logs):**
  - `tests/e2e/android_tunnel_e2e.sh` — pass/fail e2e (auto-teardown).
  - `tests/e2e/android_tunnel_debug.sh` — **persistent** rig, answer at DEBUG, frame logs
    via `docker logs`. Env: `ANDROID_SERIAL=<serial>`, `ANSWER_NET=host|bridge`,
    `ANSWER_LEVEL=debug|info`, `REBUILD=0`; `--clean` to tear down. Drive with
    `curl -s http://127.0.0.1:18080/marker.txt`.
  - `tests/e2e/lib/android_wizard.sh` — uiautomator-based wizard automation (works on
    physical devices).
- **To capture the real failure (next planned step, "Option B"):** deploy the instrumented
  `p2p-answer` (build: `cargo build --release -p p2p-answer`) at `level=debug` on
  answer-office; reconfigure the A54's wizard for answer-office; then `tcpdump -ni any
  'host <phone-public-ip> and udp'` **on answer-office** (and ideally a phone-side capture
  via PCAPdroid / root `tcpdump`) while driving requests — to see whether the offer's UDP
  DATA/SACK actually arrives at the answer host, and from which 5-tuple.

---

## 12. Reference data

- **Phones:** Samsung **SM-A546E** (`R5CW31AX4FL`, Android **16**, arm64, vnet fallback
  engaged). LG **H872 / G6** (`LGH87250967ab9`, Android **8.0** SDK 26, arm64, no
  fallback). App package `com.phillipchin.webrtctunnel`.
- **Home/laptop:** LAN `192.168.88.0/24`; laptop `192.168.88.109`; A54 Wi-Fi
  `192.168.88.106`; shared public IP `24.130.174.186`. Laptop offer peer id `offer-arisu`.
- **answer-office:** remote (coworking), **Dockerized** (advertised Docker-bridge host
  candidate `172.17.0.4`), public/srflx `162.229.61.169`.
- **A54 on T-Mobile cellular:** LTE, **IPv6-primary + 464XLAT** (CLAT v4 `192.0.0.4`),
  reported `NOT_METERED`; app still classifies it Metered and blocks by default — enable
  Settings → Network Policy → Allow metered, then tap **"Allow This Session"** on the
  paused Home screen.
- **Signaling broker:** `broker.emqx.io:8883` (MQTTS). **STUN:** `stun.l.google.com:19302`.
  **No TURN.**
- **Config format:** `p2ptunnel-config-v3`. Redaction on by default (`redact_candidates`,
  `redact_sdp`) — webrtc-rs's own `webrtc_ice` TRACE logs show un-redacted candidate
  addresses, used for §5.

## 13. Related in-repo docs (the full trail)
- `memory.md` — chronological investigation notes (entries dated `2026-06-13` "deep
  diagnosis" and `2026-06-14` "controlled rig + localization").
- `docs/ANDROID_P2P_ANSWER_DATACHANNEL_DEBUG_SPEC.md` / `..._TODO.md` — diagnostic plan.
- `docs/responses1.md` / `docs/replies1.md` — review of that plan + agreed decisions.
- `tests/e2e/README.md` — the test harness + debug rig usage.
