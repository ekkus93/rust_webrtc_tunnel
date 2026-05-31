package com.phillipchin.webrtctunnel.data

import android.content.Context
import com.phillipchin.webrtctunnel.model.LogEvent
import com.phillipchin.webrtctunnel.model.NetworkStatus
import com.phillipchin.webrtctunnel.model.TunnelStatus
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File

class DiagnosticsRepository(
    private val context: Context,
    private val configRepository: ConfigRepository,
) {
    fun exportRedactedDiagnostics(
        outputPath: String,
        status: TunnelStatus,
        logs: List<LogEvent>,
        networkStatus: NetworkStatus,
    ): Result<Unit> = runCatching {
        val output = File(outputPath)
        output.parentFile?.mkdirs()
        val payload = buildString {
            appendLine("app_version=${context.packageManager.getPackageInfo(context.packageName, 0).versionName}")
            appendLine("rust_library=p2p_mobile")
            appendLine("status_json=${Json.encodeToString(status)}")
            appendLine("network_json=${Json.encodeToString(networkStatus)}")
            appendLine("config_redacted=${configRepository.redactConfig(configRepository.readConfig())}")
            appendLine("recent_logs_redacted=${Json.encodeToString(logs.map { it.redacted() })}")
        }
        output.writeText(payload)
    }

    private fun LogEvent.redacted(): LogEvent {
        val cleaned = message
            .replace(Regex("""(?i)password[^,\s]*\s*=\s*\S+"""), "password=***REDACTED***")
            .replace(Regex("""(?i)token[^,\s]*\s*=\s*\S+"""), "token=***REDACTED***")
            .replace(Regex("""(?i)sdp[:=]\s*.*"""), "sdp=***REDACTED***")
            .replace(Regex("""(?i)candidate[:=]\s*.*"""), "candidate=***REDACTED***")
            .replace(Regex("""(?i)kex_secret\s*=\s*.*"""), "kex_secret=***REDACTED***")
            .replace(Regex("""(?i)signing_key\s*=\s*.*"""), "signing_key=***REDACTED***")
        return copy(message = cleaned)
    }
}
