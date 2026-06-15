#!/usr/bin/env bash
#
# Phase A (docker-compose variant) — full two-peer tunnel over a real TLS broker,
# with the offer and answer running as SEPARATE containers.
#
# Pipeline:
#   tester -> offer (local listener) -> WebRTC (over the compose bridge) ->
#   answer -> target (nginx) -> back
#
# Generates a throwaway CA + broker server cert, two peer identities + cross
# authorized_keys, and the two daemon configs into ./generated/ (gitignored) at
# runtime, then brings the stack up and asserts the tunnel delivers the target's
# unique marker through the offer's local listener.
#
# This is the heavier local playground; the lighter, CI-friendly equivalent is
# `cargo test -p p2p-daemon --test real_broker_tunnel`.
#
# Requires: docker + compose v2, openssl. Host-built release binaries are mounted
# into ubuntu:24.04 (matching host glibc), so no in-Docker workspace build.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
GEN="$HERE/generated"
P2PCTL="$ROOT/target/release/p2pctl"

log() { printf '\033[1;34m[e2e]\033[0m %s\n' "$*"; }
fail() { printf '\033[1;31m[e2e FAIL]\033[0m %s\n' "$*" >&2; exit 1; }

command -v docker >/dev/null || fail "docker not found"
docker compose version >/dev/null 2>&1 || fail "docker compose v2 required"
command -v openssl >/dev/null || fail "openssl not found"

cleanup() {
  log "tearing down"
  ( cd "$HERE" && docker compose down -v --remove-orphans >/dev/null 2>&1 || true )
}
trap cleanup EXIT

# --- build host binaries if missing ---
for bin in p2p-offer p2p-answer p2pctl; do
  if [ ! -x "$ROOT/target/release/$bin" ]; then
    log "building release binaries (missing $bin)"
    ( cd "$ROOT" && cargo build --release -p p2p-offer -p p2p-answer -p p2pctl )
    break
  fi
done

# --- fresh generated assets ---
rm -rf "$GEN"
mkdir -p "$GEN/certs"

MARKER="P2P-E2E-OK-$(date +%s)-$$"
printf '%s' "$MARKER" > "$GEN/marker.txt"
printf '<html><body>%s</body></html>\n' "$MARKER" > "$GEN/index.html"

# --- TLS: throwaway CA + broker server cert (SAN: broker, localhost) ---
log "generating CA + broker server cert"
openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "$GEN/certs/ca.key" -out "$GEN/certs/ca.crt" \
  -subj "/CN=p2p-e2e-ca" -days 3 -addext "basicConstraints=critical,CA:TRUE" >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes \
  -keyout "$GEN/certs/server.key" -out "$GEN/certs/server.csr" \
  -subj "/CN=broker" >/dev/null 2>&1
printf 'subjectAltName=DNS:broker,DNS:localhost,IP:127.0.0.1\nbasicConstraints=CA:FALSE\n' > "$GEN/certs/san.ext"
openssl x509 -req -in "$GEN/certs/server.csr" \
  -CA "$GEN/certs/ca.crt" -CAkey "$GEN/certs/ca.key" -CAcreateserial \
  -out "$GEN/certs/server.crt" -days 3 -extfile "$GEN/certs/san.ext" >/dev/null 2>&1

# --- identities + cross-authorized_keys ---
log "generating peer identities"
HOME="$GEN/h_offer" "$P2PCTL" keygen offer-peer --force >/dev/null
HOME="$GEN/h_answer" "$P2PCTL" keygen answer-peer --force >/dev/null
cp "$GEN/h_offer/.config/p2ptunnel/identity" "$GEN/offer-identity"
cp "$GEN/h_offer/.config/p2ptunnel/identity.pub" "$GEN/offer.pub"
cp "$GEN/h_answer/.config/p2ptunnel/identity" "$GEN/answer-identity"
cp "$GEN/h_answer/.config/p2ptunnel/identity.pub" "$GEN/answer.pub"
# Each peer authorizes the other.
cp "$GEN/answer.pub" "$GEN/offer-authorized_keys"
cp "$GEN/offer.pub" "$GEN/answer-authorized_keys"

# --- daemon configs ---
emit_config() {
  # $1=role  $2=peer_id  $3=remote_peer_id  $4=identity  $5=authorized_keys
  cat <<EOF
format = "p2ptunnel-config-v3"

[node]
peer_id = "$2"
role = "$1"

[peer]
remote_peer_id = "$3"

[paths]
identity = "/e2e/$4"
authorized_keys = "/e2e/$5"
state_dir = "/var/lib/p2p/state"
log_dir = "/var/lib/p2p/log"

[broker]
url = "mqtts://broker:8883"
client_id = "$2"
topic_prefix = "p2ptunnel-e2e"
username = ""
password_file = ""
qos = 1
keepalive_secs = 30
clean_session = false
connect_timeout_secs = 5
session_expiry_secs = 0

[broker.tls]
ca_file = "/e2e/certs/ca.crt"
client_cert_file = ""
client_key_file = ""
insecure_skip_verify = false

[webrtc]
stun_urls = []
enable_trickle_ice = false
enable_ice_restart = true
android_ice_mode = "auto"

[tunnel]
read_chunk_size = 16384
local_eof_grace_ms = 250
remote_eof_grace_ms = 250
data_plane_probe_timeout_ms = 5000

[[forwards]]
id = "web"

[forwards.offer]
listen_host = "127.0.0.1"
listen_port = 8080

[forwards.answer]
target_host = "target"
target_port = 80
allow_remote_peers = ["offer-peer"]

[reconnect]
enable_auto_reconnect = true
strategy = "ice_then_renegotiate"
ice_restart_timeout_secs = 8
renegotiate_timeout_secs = 20
backoff_initial_ms = 1000
backoff_max_ms = 30000
backoff_multiplier = 2.0
jitter_ratio = 0.20
max_attempts = 0
hold_local_client_during_reconnect = false
local_client_hold_secs = 0

[security]
require_mqtt_tls = true
require_message_encryption = true
require_message_signatures = true
require_authorized_keys = true
max_clock_skew_secs = 120
max_message_age_secs = 300
replay_cache_size = 10000
reject_unknown_config_keys = true
refuse_world_readable_identity = true
refuse_world_writable_paths = true

[logging]
level = "info"
format = "text"
file_logging = false
stdout_logging = true
log_file = "/var/lib/p2p/log/p2ptunnel.log"
redact_secrets = true
redact_sdp = true
redact_candidates = true
log_rotation = "none"

[health]
status_socket = ""
write_status_file = false
status_file = "/var/lib/p2p/state/status.json"
EOF
}

emit_config offer offer-peer answer-peer offer-identity offer-authorized_keys > "$GEN/offer.toml"
emit_config answer answer-peer offer-peer answer-identity answer-authorized_keys > "$GEN/answer.toml"

# The tester reads its script from /e2e (the generated mount), so copy it in.
cp "$HERE/tester.sh" "$GEN/tester.sh"

# --- perms: private identities not world-readable; everything traversable ---
chmod 600 "$GEN/offer-identity" "$GEN/answer-identity"
chmod 644 "$GEN/certs/ca.crt" "$GEN/certs/server.crt" "$GEN/certs/server.key" \
  "$GEN"/*.toml "$GEN"/*-authorized_keys "$GEN"/*.pub "$GEN/index.html" \
  "$GEN/marker.txt" "$GEN/tester.sh"
find "$GEN" -type d -exec chmod 755 {} +

# --- bring up + wait for the tester to finish ---
cd "$HERE"
log "starting stack (broker, target, answer, offer, tester)"
docker compose up -d --remove-orphans >/dev/null

tester_cid="$(docker compose ps -q tester)"
[ -n "$tester_cid" ] || fail "tester container did not start"

log "waiting for tester result"
code="$(docker wait "$tester_cid")"

log "tester logs:"
docker logs "$tester_cid" 2>&1 | sed 's/^/    /'

if [ "$code" != "0" ]; then
  log "FAILURE — daemon/broker logs follow:"
  docker compose logs --no-color offer answer broker 2>&1 | tail -80 | sed 's/^/    /'
  fail "docker-compose tunnel E2E failed (tester exit $code)"
fi

log "PASS — full tunnel delivered target content over a real TLS broker"
