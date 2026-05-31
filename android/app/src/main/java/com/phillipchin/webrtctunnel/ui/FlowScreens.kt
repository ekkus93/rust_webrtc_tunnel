package com.phillipchin.webrtctunnel.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@Composable
fun SetupWizardScreen(padding: PaddingValues, onStart: () -> Unit = {}) {
    ScreenSurface(padding) {
        Text("Setup Wizard")
        Spacer(Modifier.height(12.dp))
        Text("Choose Mode")
        Text("Identity")
        Text("MQTT Broker")
        Text("Remote Peer")
        Text("Forwards")
        Text("Network Policy")
        Text("Review")
        Spacer(Modifier.height(12.dp))
        Button(onClick = onStart, modifier = Modifier.fillMaxWidth()) { Text("Start Tunnel") }
    }
}

@Composable
fun NetworkPolicyScreen(padding: PaddingValues) {
    ScreenSurface(padding) {
        Text("Network Policy")
        Spacer(Modifier.height(12.dp))
        Text("Allow cellular / metered data: OFF")
        Text("Pause tunnel when cellular/metered network is detected")
        Text("Resume tunnel when unmetered Wi-Fi returns")
        Text("Show warning before allowing cellular/metered data")
    }
}

@Composable
fun ImportExportScreen(padding: PaddingValues) {
    ScreenSurface(padding) {
        Text("Import / Export")
        Spacer(Modifier.height(12.dp))
        Text("Import config file")
        Text("Import identity")
        Text("Import authorized peer/public identity")
        Text("Export config")
        Text("Export public identity")
        Text("Export diagnostics")
        Text("Export private identity")
        Spacer(Modifier.height(12.dp))
        OutlinedButton(onClick = {}, modifier = Modifier.fillMaxWidth()) { Text("Copy Public Identity") }
    }
}
