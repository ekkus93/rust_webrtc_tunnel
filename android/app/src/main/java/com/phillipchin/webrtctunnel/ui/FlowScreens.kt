package com.phillipchin.webrtctunnel.ui

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.phillipchin.webrtctunnel.model.ForwardConfig
import com.phillipchin.webrtctunnel.model.NetworkStatus
import com.phillipchin.webrtctunnel.model.NetworkType
import com.phillipchin.webrtctunnel.viewmodel.SetupStep
import com.phillipchin.webrtctunnel.viewmodel.SetupWizardState
import com.phillipchin.webrtctunnel.viewmodel.SetupViewModel

@Composable
fun SetupWizardScreen(padding: PaddingValues, vm: SetupViewModel) {
    val state by vm.state.collectAsStateWithLifecycle()
    val forwards by vm.forwards.collectAsStateWithLifecycle()
    val networkStatus by vm.networkStatus.collectAsStateWithLifecycle(
        initialValue = NetworkStatus(NetworkType.NoNetwork, false, false, false, false, "No network"),
    )
    val canAdvance = remember(state) { vm.canAdvanceFromCurrentStep() }
    val clipboard = LocalClipboardManager.current
    val importPublicIdentityLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri != null) {
            vm.importPublicIdentityFromUri(uri)
        }
    }
    var editingForward by remember { mutableStateOf<ForwardConfig?>(null) }

    ScreenSurface(padding) {
        SectionHeader("Setup Wizard", "Configure tunnel in 6 guided steps")
        Spacer(Modifier.height(12.dp))
        WizardStepper(
            steps = SetupStep.entries.map { stepLabel(it) },
            currentIndex = state.currentStep.ordinal,
        )
        Spacer(Modifier.height(12.dp))
        when (state.currentStep) {
            SetupStep.Mode -> ModeStepContent()
            SetupStep.Identity -> IdentityStepContent(vm, state)
            SetupStep.Broker -> BrokerStepContent(vm, state)
            SetupStep.Peer -> PeerStepContent(
                vm = vm,
                state = state,
                onPaste = {
                    val text = clipboard.getText()?.text.orEmpty()
                    vm.setImportPublicIdentity(text)
                    vm.validateRemotePublicIdentity()
                },
                onImportFile = { importPublicIdentityLauncher.launch(arrayOf("text/*")) },
            )
            SetupStep.Forwards -> ForwardsStepContent(vm, forwards, onAdd = {
                editingForward = ForwardConfig(
                    id = "forward_${System.currentTimeMillis()}",
                    name = "New Forward",
                    localHost = "127.0.0.1",
                    localPort = 8080,
                    remoteForwardId = "ssh",
                    enabled = true,
                )
            }, onEdit = { editingForward = it }, onDelete = vm::deleteForward)
            SetupStep.NetworkPolicy -> PolicyStepContent(vm, state, networkStatus)
            SetupStep.Review -> ReviewStepContent(vm, state, forwards)
        }
        state.brokerTestMessage?.let {
            Spacer(Modifier.height(8.dp))
            Text(it, color = MaterialTheme.colorScheme.primary)
        }
        state.errorMessage?.let {
            Spacer(Modifier.height(8.dp))
            ErrorResolutionCard(summary = it, fix = "Adjust inputs for this step and try again.")
        }
        state.saveResult?.let {
            Spacer(Modifier.height(8.dp))
            Text(it, color = Color(0xFF2E7D32))
        }
        Spacer(Modifier.height(12.dp))
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(onClick = vm::cancel) { Text("Cancel") }
                OutlinedButton(onClick = vm::goBack, enabled = state.currentStep != SetupStep.Mode) { Text("Back") }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (state.currentStep == SetupStep.Broker) {
                    OutlinedButton(onClick = vm::testBrokerConnection) { Text("Test Broker") }
                }
                if (state.currentStep == SetupStep.Review) {
                    Button(onClick = vm::saveAndApplyConfig) { Text("Save & Start Offer") }
                } else {
                    Button(onClick = vm::goNext, enabled = canAdvance) { Text("Next") }
                }
            }
        }
    }

    editingForward?.let { draft ->
        EditForwardDialog(
            initial = draft,
            onDismiss = { editingForward = null },
            onSave = { updated ->
                vm.upsertForward(updated)
                editingForward = null
            },
        )
    }
}

private fun stepLabel(step: SetupStep): String = when (step) {
    SetupStep.Mode -> "Mode"
    SetupStep.Identity -> "Identity"
    SetupStep.Broker -> "Broker"
    SetupStep.Peer -> "Peer"
    SetupStep.Forwards -> "Forwards"
    SetupStep.NetworkPolicy -> "Policy"
    SetupStep.Review -> "Review"
}

@Composable
private fun ModeStepContent() {
    StatusCard {
        Text("Tunnel mode")
        Text("Offer mode is enabled on Android.")
        OutlinedButton(onClick = {}, enabled = false, modifier = Modifier.fillMaxWidth()) {
            Text("Answer mode unavailable in Android v1")
        }
    }
}

@Composable
private fun IdentityStepContent(vm: SetupViewModel, state: SetupWizardState) {
    val context = LocalContext.current
    val clipboard = LocalClipboardManager.current
    StatusCard {
        OutlinedTextField(value = state.input.localPeerId, onValueChange = { vm.setInput(state.input.copy(localPeerId = it)) }, label = { Text("Local peer id") }, modifier = Modifier.fillMaxWidth())
        OutlinedTextField(value = state.importIdentityPath, onValueChange = vm::setImportIdentityPath, label = { Text("Private identity path") }, modifier = Modifier.fillMaxWidth())
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = vm::importIdentityFromPath, modifier = Modifier.weight(1f)) { Text("Import identity") }
            OutlinedButton(onClick = vm::generateIdentity, modifier = Modifier.weight(1f)) { Text("Generate identity") }
        }
        if (state.localPublicIdentity.isNotBlank()) {
            Text("Local public identity:")
            Text(state.localPublicIdentity, style = MaterialTheme.typography.bodySmall)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedButton(
                    onClick = { clipboard.setText(AnnotatedString(state.localPublicIdentity)) },
                    modifier = Modifier.weight(1f),
                ) { Text("Copy Public Key") }
                OutlinedButton(
                    onClick = {
                        val share = android.content.Intent(android.content.Intent.ACTION_SEND).apply {
                            type = "text/plain"
                            putExtra(android.content.Intent.EXTRA_SUBJECT, "WebRTC Tunnel public identity")
                            putExtra(android.content.Intent.EXTRA_TEXT, state.localPublicIdentity)
                        }
                        context.startActivity(android.content.Intent.createChooser(share, "Share public identity").addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK))
                    },
                    modifier = Modifier.weight(1f),
                ) { Text("Share Public Key") }
            }
        }
    }
}

@Composable
private fun BrokerStepContent(vm: SetupViewModel, state: SetupWizardState) {
    StatusCard {
        OutlinedTextField(value = state.input.brokerHost, onValueChange = { vm.setInput(state.input.copy(brokerHost = it)) }, label = { Text("Broker host") }, modifier = Modifier.fillMaxWidth())
        OutlinedTextField(value = state.input.brokerPort.toString(), onValueChange = { value -> vm.setInput(state.input.copy(brokerPort = value.toIntOrNull() ?: 0)) }, label = { Text("Broker port") }, modifier = Modifier.fillMaxWidth())
        OutlinedTextField(value = state.input.brokerUsername, onValueChange = { vm.setInput(state.input.copy(brokerUsername = it)) }, label = { Text("Broker username") }, modifier = Modifier.fillMaxWidth())
        OutlinedTextField(
            value = state.input.brokerPassword,
            onValueChange = { vm.setInput(state.input.copy(brokerPassword = it)) },
            label = { Text("Broker password") },
            modifier = Modifier.fillMaxWidth(),
            visualTransformation = PasswordVisualTransformation(),
        )
        OutlinedTextField(value = state.input.topicPrefix, onValueChange = { vm.setInput(state.input.copy(topicPrefix = it)) }, label = { Text("Topic prefix") }, modifier = Modifier.fillMaxWidth())
        OutlinedButton(onClick = { vm.setAdvancedExpanded(!state.advancedExpanded) }, modifier = Modifier.fillMaxWidth()) {
            Text(if (state.advancedExpanded) "Hide advanced" else "Show advanced")
        }
        if (state.advancedExpanded) {
            OutlinedTextField(value = state.input.brokerPasswordFile, onValueChange = { vm.setInput(state.input.copy(brokerPasswordFile = it)) }, label = { Text("Broker password file (advanced)") }, modifier = Modifier.fillMaxWidth())
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text("Use TLS")
                Spacer(Modifier.weight(1f))
                Switch(checked = state.input.brokerUseTls, onCheckedChange = { vm.setInput(state.input.copy(brokerUseTls = it)) })
            }
        }
    }
}

@Composable
private fun PeerStepContent(
    vm: SetupViewModel,
    state: SetupWizardState,
    onPaste: () -> Unit,
    onImportFile: () -> Unit,
) {
    StatusCard {
        OutlinedTextField(value = state.input.remotePeerId, onValueChange = { vm.setInput(state.input.copy(remotePeerId = it)) }, label = { Text("Remote peer id") }, modifier = Modifier.fillMaxWidth())
        OutlinedTextField(value = state.importPublicIdentity, onValueChange = vm::setImportPublicIdentity, label = { Text("Remote public identity") }, modifier = Modifier.fillMaxWidth())
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = vm::validateRemotePublicIdentity, modifier = Modifier.weight(1f)) { Text("Validate remote identity") }
            OutlinedButton(onClick = {}, enabled = false, modifier = Modifier.weight(1f)) { Text("Answer mode disabled") }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(onClick = onPaste, modifier = Modifier.weight(1f)) { Text("Paste from clipboard") }
            OutlinedButton(onClick = onImportFile, modifier = Modifier.weight(1f)) { Text("Import from file") }
        }
        Text("The answer side must authorize this phone's public identity.")
    }
}

@Composable
private fun ForwardsStepContent(
    vm: SetupViewModel,
    forwards: List<ForwardConfig>,
    onAdd: () -> Unit,
    onEdit: (ForwardConfig) -> Unit,
    onDelete: (String) -> Unit,
) {
    StatusCard {
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text("Forward rules", style = MaterialTheme.typography.titleMedium)
            IconButton(onClick = onAdd) { Icon(Icons.Default.Add, "Add forward") }
        }
        if (forwards.isEmpty()) {
            Text("No forwards configured.")
        } else {
            LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                items(forwards) { forward ->
                    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text(forward.name, style = MaterialTheme.typography.titleSmall)
                            Text("${forward.localHost}:${forward.localPort} -> ${forward.remoteForwardId}")
                        }
                        Row {
                            OutlinedButton(onClick = { onEdit(forward) }) { Text("Edit") }
                            IconButton(onClick = { onDelete(forward.id) }) { Icon(Icons.Default.Delete, "Delete forward") }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun PolicyStepContent(vm: SetupViewModel, state: SetupWizardState, networkStatus: NetworkStatus) {
    StatusCard {
        Text("Current network: ${networkStatus.networkType}")
        Text(if (networkStatus.isMetered) "Metered" else "Unmetered")
        Text(if (networkStatus.tunnelAllowed) "Tunnel allowed" else "Tunnel blocked")
        networkStatus.blockReason?.let { Text("Reason: $it") }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Allow metered / cellular network")
            Spacer(Modifier.weight(1f))
            Switch(checked = state.input.allowMetered, onCheckedChange = { vm.setInput(state.input.copy(allowMetered = it)) })
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Resume on unmetered")
            Spacer(Modifier.weight(1f))
            Switch(checked = state.input.resumeOnUnmetered, onCheckedChange = { vm.setInput(state.input.copy(resumeOnUnmetered = it)) })
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Acknowledge non-localhost bind warning")
            Spacer(Modifier.weight(1f))
            Switch(checked = state.nonLocalhostWarningAccepted, onCheckedChange = vm::setNonLocalhostWarningAccepted)
        }
        Text("Non-localhost bind remains advanced and warning-gated.")
    }
}

@Composable
private fun ReviewStepContent(vm: SetupViewModel, state: SetupWizardState, forwards: List<ForwardConfig>) {
    StatusCard {
        Text("Mode: Offer")
        Text("Local peer: ${state.input.localPeerId}")
        Text("Remote peer: ${state.input.remotePeerId}")
        Text("Broker: ${state.input.brokerHost}:${state.input.brokerPort}")
        Text("Topic prefix: ${state.input.topicPrefix}")
        Text("Remote identity imported: ${state.remoteIdentityPeerId ?: "No"}")
        Text("Forwards enabled: ${forwards.count { it.enabled }} / ${forwards.size}")
        Text("Allow metered: ${state.input.allowMetered}")
        Text("Resume on unmetered: ${state.input.resumeOnUnmetered}")
        OutlinedButton(onClick = vm::startTunnelFromReview, modifier = Modifier.fillMaxWidth()) { Text("Start tunnel") }
    }
}

@Composable
internal fun EditForwardDialog(initial: ForwardConfig, onDismiss: () -> Unit, onSave: (ForwardConfig) -> Unit) {
    var value by remember(initial) { mutableStateOf(initial) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (initial.id == value.id) "Edit Forward" else "Add Forward") },
        text = {
            LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                item { OutlinedTextField(value = value.name, onValueChange = { value = value.copy(name = it) }, label = { Text("Display name") }, modifier = Modifier.fillMaxWidth()) }
                item { OutlinedTextField(value = value.localHost, onValueChange = { value = value.copy(localHost = it) }, label = { Text("Local host") }, modifier = Modifier.fillMaxWidth()) }
                item { OutlinedTextField(value = value.localPort.toString(), onValueChange = { value = value.copy(localPort = it.toIntOrNull() ?: 0) }, label = { Text("Local port") }, modifier = Modifier.fillMaxWidth()) }
                item { OutlinedTextField(value = value.remoteForwardId, onValueChange = { value = value.copy(remoteForwardId = it) }, label = { Text("Remote forward_id") }, modifier = Modifier.fillMaxWidth()) }
                item {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text("Enabled")
                        Spacer(Modifier.weight(1f))
                        Switch(checked = value.enabled, onCheckedChange = { value = value.copy(enabled = it) })
                    }
                }
            }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
        confirmButton = { Button(onClick = { onSave(value) }) { Text("Save") } },
    )
}
