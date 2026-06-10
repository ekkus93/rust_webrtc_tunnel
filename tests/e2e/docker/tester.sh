#!/bin/sh
# Runs in the OFFER container's network namespace, so the offer's local forward
# listener is reachable on 127.0.0.1:8080. Retries an HTTP GET through the tunnel
# until it returns the target service's unique marker, proving data flowed:
#   curl -> offer listener -> WebRTC -> answer -> target (nginx) -> back.
set -eu
MARKER="$(cat /e2e/marker.txt)"
echo "tester: waiting for tunnel to deliver marker=$MARKER"
i=0
while [ "$i" -lt 60 ]; do
  body="$(curl -s --max-time 5 http://127.0.0.1:8080/ 2>/dev/null || true)"
  case "$body" in
    *"$MARKER"*)
      echo "tester: PASS — tunnel delivered target content"
      exit 0
      ;;
  esac
  i=$((i + 1))
  sleep 2
done
echo "tester: FAIL — tunnel did not deliver target content within timeout"
exit 1
