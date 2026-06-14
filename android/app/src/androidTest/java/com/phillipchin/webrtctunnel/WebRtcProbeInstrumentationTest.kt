package com.phillipchin.webrtctunnel

import android.util.Log
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Drives the native WebRTC self-diagnostic ([RustValidationBridge.webrtcProbe]) on
 * the device/emulator. It needs no broker, no remote peer, and no NAT traversal, so
 * it can answer — directly on Android — whether `p2p-webrtc`:
 *  - gathers a `host` ICE candidate with the device's LAN IP (and a STUN `srflx`), and
 *  - can complete a full ICE + DTLS + data-channel handshake (loopback) on-device.
 *
 * The full report is logged under the "WebRtcProbe" tag for inspection.
 */
@RunWith(AndroidJUnit4::class)
class WebRtcProbeInstrumentationTest {
    @Serializable
    private data class Report(
        @SerialName("os_local_ip") val osLocalIp: String? = null,
        val interfaces: List<String> = emptyList(),
        val gather: Gather = Gather(),
        val loopback: Loopback = Loopback(),
    )

    @Serializable
    private data class Gather(
        val ok: Boolean = false,
        val error: String? = null,
        val host: Int = 0,
        val srflx: Int = 0,
        val relay: Int = 0,
        val other: Int = 0,
        val candidates: List<String> = emptyList(),
    )

    @Serializable
    private data class Loopback(
        val ok: Boolean = false,
        val detail: String = "",
        @SerialName("elapsed_ms") val elapsedMs: Long = 0,
    )

    @Test
    fun webrtcProbeGathersCandidatesAndCompletesLoopback() {
        val json = RustWebRtcProbe().probe(PROBE_TIMEOUT_SECS)
        Log.i(TAG, "raw report: $json")
        val report = Json { ignoreUnknownKeys = true }.decodeFromString<Report>(json)

        Log.i(TAG, "os_local_ip=${report.osLocalIp}")
        Log.i(TAG, "webrtc-rs interfaces (${report.interfaces.size}):")
        report.interfaces.forEach { Log.i(TAG, "  iface: $it") }
        Log.i(
            TAG,
            "gather ok=${report.gather.ok} host=${report.gather.host} " +
                "srflx=${report.gather.srflx} relay=${report.gather.relay} " +
                "other=${report.gather.other} error=${report.gather.error}",
        )
        report.gather.candidates.forEach { Log.i(TAG, "  candidate: $it") }
        Log.i(
            TAG,
            "loopback ok=${report.loopback.ok} elapsed_ms=${report.loopback.elapsedMs} " +
                "detail=${report.loopback.detail}",
        )

        Log.i(
            TAG,
            "VERDICT os_local_ip=${report.osLocalIp} host_candidate_gathered=${report.gather.host > 0} " +
                "loopback_ok=${report.loopback.ok}",
        )

        // Regression guard for the Android host-candidate fix: even though the OS
        // interface enumeration is restricted on Android (ifaces() errors), the
        // SettingEngine fallback in p2p-webrtc must still produce a host candidate and
        // let a full ICE + DTLS + data-channel handshake complete on-device.
        assertTrue("candidate gathering errored: ${report.gather.error}", report.gather.error == null)
        assertTrue(
            "no host ICE candidate gathered despite os_local_ip=${report.osLocalIp} " +
                "(host=${report.gather.host} srflx=${report.gather.srflx})",
            report.gather.host > 0,
        )
        assertTrue(
            "on-device loopback handshake failed: ${report.loopback.detail}",
            report.loopback.ok,
        )
    }

    private companion object {
        const val TAG = "WebRtcProbe"
        const val PROBE_TIMEOUT_SECS = 10L
    }
}
