package com.phillipchin.webrtctunnel.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.phillipchin.webrtctunnel.model.ForwardConfig
import com.phillipchin.webrtctunnel.model.NetworkType

// Local ports that usually serve HTTP-like content (offer "Open in browser" hint).
private const val PORT_HTTP = 80
private const val PORT_HTTP_8080 = 8080
private const val PORT_HTTP_8000 = 8000
private const val PORT_DEV_3000 = 3000
private const val PORT_FLASK_5000 = 5000
private const val PORT_VITE = 5173
private const val PORT_GRADIO = 7860
private const val PORT_OLLAMA = 11434
private val HTTP_LIKE_PORTS =
    setOf(
        PORT_HTTP,
        PORT_HTTP_8080,
        PORT_HTTP_8000,
        PORT_DEV_3000,
        PORT_FLASK_5000,
        PORT_VITE,
        PORT_GRADIO,
        PORT_OLLAMA,
    )

internal fun isBrowserOpenable(forward: ForwardConfig): Boolean {
    val name = "${forward.name} ${forward.remoteForwardId}".lowercase()
    if (forward.localPort in HTTP_LIKE_PORTS) return true
    return listOf("http", "web", "api", "llama", "ollama").any { token -> name.contains(token) }
}

/** Display/copy address for a local forward, using the host exactly as configured. */
internal fun localForwardAddress(forward: ForwardConfig): String = "${forward.localHost}:${forward.localPort}"

/**
 * Host to open in a browser. Wildcard/unspecified bind addresses are normalized to
 * loopback since they are not reachable as a destination. Validation currently only
 * permits `127.0.0.1`/`localhost`, so the wildcard cases are future-proofing.
 */
internal fun browserHostForLocalForward(host: String): String =
    when (host.trim().lowercase()) {
        "", "0.0.0.0", "::", "[::]" -> "127.0.0.1"
        else -> host.trim()
    }

/** Browser URL for a local forward, with the host normalized for reachability. */
internal fun browserUrlForForward(forward: ForwardConfig): String =
    "http://${browserHostForLocalForward(forward.localHost)}:${forward.localPort}"

internal fun mapNetworkTypeLabel(networkType: NetworkType): String =
    when (networkType) {
        NetworkType.UnmeteredWifi -> "Wi-Fi"
        NetworkType.MeteredWifi -> "Metered Wi-Fi"
        NetworkType.Cellular -> "Cellular"
        NetworkType.NoNetwork -> "No network"
        NetworkType.Unknown -> "Unknown"
    }

internal fun mapForwardListenLabel(state: String): String =
    when (state.lowercase()) {
        "listening" -> "Listening"
        "stopped" -> "Stopped"
        "error" -> "Error"
        "disabled" -> "Disabled"
        "paused" -> "Paused"
        "configured" -> "Configured"
        else -> state
    }

@Composable
internal fun PreferenceSwitch(
    title: String,
    checked: Boolean,
    onToggle: (Boolean) -> Unit,
) {
    Row(
        modifier =
            Modifier
                .fillMaxWidth()
                .height(48.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(title, modifier = Modifier.weight(1f))
        Switch(checked = checked, onCheckedChange = onToggle)
    }
}
