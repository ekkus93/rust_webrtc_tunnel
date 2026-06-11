package com.phillipchin.webrtctunnel.ui

import android.content.Intent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.phillipchin.webrtctunnel.BuildConfig
import com.phillipchin.webrtctunnel.model.AndroidAppPreferences
import com.phillipchin.webrtctunnel.viewmodel.SettingsViewModel

private const val IDENTITY_DISPLAY_MAX = 28
private const val IDENTITY_PREFIX_CHARS = 16
private const val IDENTITY_SUFFIX_CHARS = 8

private fun truncateIdentity(key: String): String =
    if (key.length > IDENTITY_DISPLAY_MAX) {
        "${key.take(IDENTITY_PREFIX_CHARS)}…${key.takeLast(IDENTITY_SUFFIX_CHARS)}"
    } else {
        key
    }

data class SettingsNavActions(
    val onOpenSetup: () -> Unit,
    val onOpenLogs: () -> Unit,
    val onOpenNetworkPolicy: () -> Unit,
    val onOpenImportExport: () -> Unit,
)

@Composable
fun SettingsScreen(
    padding: PaddingValues,
    vm: SettingsViewModel,
    nav: SettingsNavActions,
) {
    val onOpenSetup = nav.onOpenSetup
    val onOpenLogs = nav.onOpenLogs
    val onOpenNetworkPolicy = nav.onOpenNetworkPolicy
    val onOpenImportExport = nav.onOpenImportExport
    val prefs by vm.preferences.collectAsStateWithLifecycle(initialValue = AndroidAppPreferences())
    val uiState by vm.uiState.collectAsStateWithLifecycle()
    val context = LocalContext.current
    val clipboard = LocalClipboardManager.current
    val publicIdentity = uiState.publicIdentity
    val hasPublicIdentity = !publicIdentity.isNullOrBlank()
    var showMeteredWarningDialog by remember { mutableStateOf(false) }
    var showResetConfirmDialog by remember { mutableStateOf(false) }
    ScrollableScreenSurface(padding) {
        SectionHeader("Settings", "Tunnel and app behavior")
        Spacer(Modifier.height(12.dp))
        SettingsSection("Tunnel") {
            PreferenceSwitch("Start tunnel when app opens", prefs.startTunnelWhenAppOpens) {
                vm.savePreferences(prefs.copy(startTunnelWhenAppOpens = it))
            }
            PreferenceSwitch("Resume tunnel when Wi-Fi returns", prefs.resumeOnUnmetered) {
                vm.savePreferences(prefs.copy(resumeOnUnmetered = it))
            }
            OutlinedButton(onClick = onOpenSetup, modifier = Modifier.fillMaxWidth()) { Text("Run setup wizard again") }
        }
        Spacer(Modifier.height(12.dp))
        SettingsSection("Network Policy") {
            Text(
                "Cellular / metered: ${if (prefs.allowMetered) "Allowed" else "Blocked"}",
                style = MaterialTheme.typography.bodySmall,
                color = Color(color = 0xFF6B7280),
            )
            OutlinedButton(onClick = onOpenNetworkPolicy, modifier = Modifier.fillMaxWidth()) {
                Text("Open network policy details")
            }
        }
        Spacer(Modifier.height(12.dp))
        SettingsSection("Configuration") {
            OutlinedButton(
                onClick = { vm.validateConfig() },
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Validate configuration") }
            DestructiveActionButton("Reset configuration") { showResetConfirmDialog = true }
        }
        Spacer(Modifier.height(12.dp))
        SettingsSection("Identity") {
            Text(
                if (publicIdentity != null) truncateIdentity(publicIdentity) else "No local public identity found.",
                style = MaterialTheme.typography.bodySmall,
            )
            uiState.publicIdentityLoadError?.let { error ->
                Text(error, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(
                    onClick = {
                        clipboard.setText(AnnotatedString(publicIdentity.orEmpty()))
                    },
                    modifier = Modifier.weight(1f),
                    enabled = hasPublicIdentity,
                ) { Text("Copy identity") }
                OutlinedButton(
                    onClick = {
                        val share =
                            Intent(Intent.ACTION_SEND).apply {
                                type = "text/plain"
                                putExtra(Intent.EXTRA_SUBJECT, "WebRTC Tunnel public identity")
                                putExtra(Intent.EXTRA_TEXT, publicIdentity)
                            }
                        context.startActivity(
                            Intent.createChooser(
                                share,
                                "Share public identity",
                            ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
                        )
                    },
                    modifier = Modifier.weight(1f),
                    enabled = hasPublicIdentity,
                ) { Text("Share identity") }
            }
            OutlinedButton(
                onClick = onOpenImportExport,
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Import / Export identity") }
        }
        Spacer(Modifier.height(12.dp))
        SettingsSection("Diagnostics") {
            OutlinedButton(
                onClick = onOpenLogs,
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Open logs / export diagnostics") }
            OutlinedButton(
                onClick = {
                    val share =
                        Intent.createChooser(vm.diagnosticsShareIntent(), "Share diagnostics")
                            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                    context.startActivity(share)
                },
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Share diagnostics") }
        }
        Spacer(Modifier.height(12.dp))
        SettingsSection("Advanced") {
            OutlinedButton(
                onClick = { vm.savePreferences(prefs.copy(advancedSettingsEnabled = !prefs.advancedSettingsEnabled)) },
                modifier = Modifier.fillMaxWidth(),
            ) { Text(if (prefs.advancedSettingsEnabled) "Hide advanced settings" else "Show advanced settings") }
            if (prefs.advancedSettingsEnabled) {
                PreferenceSwitch(
                    "Enable debug logs",
                    prefs.debugLogsEnabled,
                ) { vm.savePreferences(prefs.copy(debugLogsEnabled = it)) }
                OutlinedButton(
                    onClick = onOpenSetup,
                    modifier = Modifier.fillMaxWidth(),
                ) { Text("Edit custom topic prefix") }
                OutlinedButton(
                    onClick = onOpenSetup,
                    modifier = Modifier.fillMaxWidth(),
                ) { Text("Configure non-localhost bind (advanced)") }
                Text(
                    "Answer mode: not available on Android",
                    style = MaterialTheme.typography.bodySmall,
                    color = Color(color = 0xFF6B7280),
                )
                OutlinedButton(
                    onClick = { clipboard.setText(AnnotatedString(vm.statusJson())) },
                    modifier = Modifier.fillMaxWidth(),
                ) { Text("Copy status JSON") }
                OutlinedButton(
                    onClick = { clipboard.setText(AnnotatedString(vm.redactedConfigOrEmpty())) },
                    modifier = Modifier.fillMaxWidth(),
                ) { Text("Copy redacted config") }
            }
        }
        Spacer(Modifier.height(12.dp))
        SettingsSection("About") {
            Text("Rust WebRTC Tunnel Android", style = MaterialTheme.typography.bodyMedium)
            Text(
                "Version ${BuildConfig.VERSION_NAME}",
                style = MaterialTheme.typography.bodySmall,
                color = Color(color = 0xFF6B7280),
            )
        }
    }
    if (showMeteredWarningDialog) {
        MeteredWarningDialog(
            onConfirm = {
                vm.savePreferences(prefs.copy(allowMetered = true))
                showMeteredWarningDialog = false
            },
            onDismiss = { showMeteredWarningDialog = false },
        )
    }
    if (showResetConfirmDialog) {
        AlertDialog(
            onDismissRequest = { showResetConfirmDialog = false },
            title = { Text("Reset configuration?") },
            text = {
                Text(
                    "This clears all saved configuration including broker, peer, and forwards. This cannot be undone.",
                )
            },
            dismissButton = { TextButton(onClick = { showResetConfirmDialog = false }) { Text("Cancel") } },
            confirmButton = {
                TextButton(onClick = {
                    vm.resetConfiguration()
                    showResetConfirmDialog = false
                }) { Text("Reset", color = MaterialTheme.colorScheme.error) }
            },
        )
    }
}
