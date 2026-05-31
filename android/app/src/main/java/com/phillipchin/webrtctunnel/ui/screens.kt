package com.phillipchin.webrtctunnel.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.phillipchin.webrtctunnel.model.ServiceState
import com.phillipchin.webrtctunnel.model.TunnelMode
import com.phillipchin.webrtctunnel.model.TunnelStatus
import com.phillipchin.webrtctunnel.viewmodel.ForwardsViewModel
import com.phillipchin.webrtctunnel.viewmodel.HomeViewModel
import com.phillipchin.webrtctunnel.viewmodel.LogsViewModel
import com.phillipchin.webrtctunnel.viewmodel.SettingsViewModel
import com.phillipchin.webrtctunnel.viewmodel.SetupViewModel

@Composable
fun HomeScreen(padding: PaddingValues, vm: HomeViewModel) {
    val status by vm.status.collectAsStateWithLifecycle()
    ScreenSurface(padding) {
        StatusCard(status)
        Spacer(Modifier.height(12.dp))
        NetworkCard(status)
        Spacer(Modifier.height(12.dp))
        ForwardsCard(status)
        Spacer(Modifier.height(12.dp))
        ActionRow(status, onStart = { vm.startTunnel(status.mode) }, onStop = vm::stopTunnel)
    }
}

@Composable
fun ForwardsScreen(padding: PaddingValues, vm: ForwardsViewModel) {
    val status by vm.status.collectAsStateWithLifecycle()
    ScreenSurface(padding) {
        Text("Forwards", style = MaterialTheme.typography.headlineSmall)
        Spacer(Modifier.height(12.dp))
        status.forwards.forEach { forward ->
            Card(Modifier.fillMaxWidth().padding(vertical = 4.dp)) {
                Column(Modifier.padding(16.dp)) {
                    Text(forward.name, style = MaterialTheme.typography.titleMedium)
                    Text("${forward.localHost}:${forward.localPort} -> ${forward.remoteForwardId}")
                    Text("Status: ${forward.listenState}")
                }
            }
        }
    }
}

@Composable
fun LogsScreen(padding: PaddingValues, vm: LogsViewModel) {
    val logs = vm.logs(50)
    ScreenSurface(padding) {
        Text("Logs", style = MaterialTheme.typography.headlineSmall)
        Spacer(Modifier.height(12.dp))
        LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(logs) { event ->
                Card(Modifier.fillMaxWidth()) {
                    Text("${event.unixMs} ${event.level.uppercase()} ${event.message}", modifier = Modifier.padding(16.dp))
                }
            }
        }
    }
}

@Composable
fun SettingsScreen(padding: PaddingValues, vm: SettingsViewModel, setupVm: SetupViewModel) {
    ScreenSurface(padding) {
        Text("Settings", style = MaterialTheme.typography.headlineSmall)
        Spacer(Modifier.height(12.dp))
        Text("Tunnel")
        Text("Network Policy")
        Text("Identity")
        Text("Configuration")
        Text("Diagnostics")
        Text("Advanced")
        Text("About")
        Spacer(Modifier.height(12.dp))
        OutlinedButton(onClick = { setupVm.validateConfig() }) { Text("Run Setup Wizard Again") }
    }
}

@Composable
fun ScreenSurface(padding: PaddingValues, content: @Composable ColumnScope.() -> Unit) {
    Column(
        modifier = Modifier.fillMaxSize().padding(padding).padding(16.dp),
        verticalArrangement = Arrangement.Top,
        content = content,
    )
}

@Composable
private fun StatusCard(status: TunnelStatus) {
    ElevatedCard(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp)) {
            Text(status.serviceState.name, style = MaterialTheme.typography.headlineSmall)
            Text("Mode: ${status.mode}")
            Text("Remote peer: ${status.remotePeerId ?: "-"}")
            Text("Active sessions: ${status.activeSessionCount}")
        }
    }
}

@Composable
private fun NetworkCard(status: TunnelStatus) {
    Card(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp)) {
            Text(status.networkStatus.networkType.name)
            Text(if (status.networkStatus.tunnelAllowed) "Tunnel allowed" else status.networkStatus.blockReason ?: "Blocked")
        }
    }
}

@Composable
private fun ForwardsCard(status: TunnelStatus) {
    Card(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp)) {
            Text("Forwards (${status.forwards.size})")
            status.forwards.forEach { forward ->
                Text("${forward.name} ${forward.localHost}:${forward.localPort} -> ${forward.remoteForwardId}")
            }
        }
    }
}

@Composable
private fun ActionRow(status: TunnelStatus, onStart: () -> Unit, onStop: () -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Button(onClick = onStart, modifier = Modifier.fillMaxWidth()) { Text("Start Tunnel") }
        OutlinedButton(onClick = onStop, modifier = Modifier.fillMaxWidth()) { Text("Stop Tunnel") }
    }
}
