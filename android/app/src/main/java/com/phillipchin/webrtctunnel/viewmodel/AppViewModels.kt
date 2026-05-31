package com.phillipchin.webrtctunnel.viewmodel

import android.content.Intent
import androidx.core.content.ContextCompat
import androidx.lifecycle.ViewModel
import com.phillipchin.webrtctunnel.TunnelForegroundService
import com.phillipchin.webrtctunnel.data.AppDependencies
import com.phillipchin.webrtctunnel.model.ForwardConfig
import com.phillipchin.webrtctunnel.model.LogEvent
import com.phillipchin.webrtctunnel.model.SetupConfigInput
import com.phillipchin.webrtctunnel.model.TunnelMode
import com.phillipchin.webrtctunnel.model.TunnelStatus
import com.phillipchin.webrtctunnel.model.ValidationResult
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking

enum class SetupStep {
    Mode,
    Identity,
    Broker,
    Peer,
    Forwards,
    NetworkPolicy,
    Review,
}

data class SetupWizardState(
    val currentStep: SetupStep = SetupStep.Mode,
    val input: SetupConfigInput = SetupConfigInput(),
    val importIdentityPath: String = "",
    val importPublicIdentity: String = "",
    val errorMessage: String? = null,
    val saveResult: String? = null,
)

data class ImportExportState(
    val configImportPath: String = "",
    val privateIdentityImportPath: String = "",
    val publicIdentityLine: String = "",
    val configExportPath: String = "",
    val publicIdentityExportPath: String = "",
    val privateIdentityExportPath: String = "",
    val diagnosticsExportPath: String = "",
    val confirmPrivateExportRisk: Boolean = false,
    val resultMessage: String? = null,
)

class HomeViewModel(private val deps: AppDependencies) : ViewModel() {
    val status: StateFlow<TunnelStatus> = deps.tunnelRepository.status

    fun startTunnel(mode: TunnelMode): Unit {
        val action = when (mode) {
            TunnelMode.Offer -> TunnelForegroundService.ACTION_START_OFFER
            TunnelMode.Answer -> return
        }
        ContextCompat.startForegroundService(
            deps.context,
            Intent(deps.context, TunnelForegroundService::class.java).setAction(action),
        )
    }

    fun stopTunnel(): Unit {
        deps.context.startService(
            Intent(deps.context, TunnelForegroundService::class.java)
                .setAction(TunnelForegroundService.ACTION_STOP),
        )
    }

    fun refresh() = deps.tunnelRepository.refreshStatus()
}

class SetupViewModel(private val deps: AppDependencies) : ViewModel() {
    private val _state = MutableStateFlow(SetupWizardState())
    val state: StateFlow<SetupWizardState> = _state.asStateFlow()
    private val steps = SetupStep.entries

    fun validateConfig(): ValidationResult = deps.tunnelRepository.validateConfig(deps.configRepository.configPath)

    fun setInput(update: SetupConfigInput) {
        _state.value = _state.value.copy(input = update, errorMessage = null, saveResult = null)
    }

    fun setImportIdentityPath(path: String) {
        _state.value = _state.value.copy(importIdentityPath = path, errorMessage = null)
    }

    fun setImportPublicIdentity(value: String) {
        _state.value = _state.value.copy(importPublicIdentity = value, errorMessage = null)
    }

    fun goBack() {
        val current = _state.value.currentStep
        val index = steps.indexOf(current)
        if (index > 0) {
            _state.value = _state.value.copy(currentStep = steps[index - 1], errorMessage = null)
        }
    }

    fun goNext() {
        val current = _state.value
        val validationError = validateStep(current.currentStep, current)
        if (validationError != null) {
            _state.value = current.copy(errorMessage = validationError)
            return
        }
        val index = steps.indexOf(current.currentStep)
        if (index < steps.lastIndex) {
            _state.value = current.copy(currentStep = steps[index + 1], errorMessage = null)
        }
    }

    fun loadSavedForwards(): List<ForwardConfig> = deps.configRepository.loadForwards()

    fun saveAndApplyConfig() {
        val current = _state.value
        val input = current.input
        val forwards = deps.configRepository.loadForwards().filter { it.enabled }
        val validationError = validateStep(SetupStep.Review, current)
        if (validationError != null) {
            _state.value = current.copy(errorMessage = validationError, saveResult = null)
            return
        }
        if (current.importIdentityPath.isNotBlank()) {
            val imported = deps.identityRepository.importPrivateIdentityFromPath(current.importIdentityPath)
            if (imported.isFailure) {
                _state.value = current.copy(
                    errorMessage = imported.exceptionOrNull()?.message ?: "Failed importing private identity",
                    saveResult = null,
                )
                return
            }
        }
        if (current.importPublicIdentity.isNotBlank()) {
            val imported = deps.identityRepository.appendAuthorizedPublicIdentity(current.importPublicIdentity)
            if (imported.isFailure) {
                _state.value = current.copy(
                    errorMessage = imported.exceptionOrNull()?.message ?: "Failed importing public identity",
                    saveResult = null,
                )
                return
            }
        }
        deps.configRepository.writeConfig(deps.configRepository.renderOfferConfig(input, forwards))
        runBlocking {
            val existing = deps.configRepository.preferences.first()
            deps.configRepository.savePreferences(
                existing.copy(
                    allowMetered = input.allowMetered,
                    resumeOnUnmetered = input.resumeOnUnmetered,
                ),
            )
        }
        val result = validateConfig()
        if (!result.valid) {
            _state.value = current.copy(errorMessage = result.message ?: "Config validation failed", saveResult = null)
            return
        }
        _state.value = current.copy(errorMessage = null, saveResult = "Configuration saved")
    }

    private fun validateStep(step: SetupStep, state: SetupWizardState): String? {
        val input = state.input
        return when (step) {
            SetupStep.Mode -> null
            SetupStep.Identity -> {
                val hasStored = deps.identityRepository.hasEncryptedIdentity()
                if (!hasStored && state.importIdentityPath.isBlank()) "Import a private identity to continue" else null
            }
            SetupStep.Broker -> when {
                input.brokerHost.isBlank() -> "Broker host is required"
                input.brokerPort !in 1..65535 -> "Broker port must be between 1 and 65535"
                else -> null
            }
            SetupStep.Peer -> {
                if (input.remotePeerId.isBlank()) "Remote peer id is required"
                else if (state.importPublicIdentity.isBlank()) "Remote public identity is required"
                else null
            }
            SetupStep.Forwards -> deps.configRepository.validateForwards(deps.configRepository.loadForwards())
                ?: if (deps.configRepository.loadForwards().none { it.enabled }) "Enable at least one forward" else null
            SetupStep.NetworkPolicy -> null
            SetupStep.Review -> {
                validateStep(SetupStep.Identity, state)
                    ?: validateStep(SetupStep.Broker, state)
                    ?: validateStep(SetupStep.Peer, state)
                    ?: validateStep(SetupStep.Forwards, state)
            }
        }
    }
}

class ForwardsViewModel(private val deps: AppDependencies) : ViewModel() {
    val status: StateFlow<TunnelStatus> = deps.tunnelRepository.status
    private val _forwards = MutableStateFlow(deps.configRepository.loadForwards())
    val forwards: StateFlow<List<ForwardConfig>> = _forwards.asStateFlow()
    private val _message = MutableStateFlow<String?>(null)
    val message: StateFlow<String?> = _message.asStateFlow()

    fun reload() {
        _forwards.value = deps.configRepository.loadForwards()
    }

    fun saveForward(forward: ForwardConfig) {
        val result = deps.configRepository.upsertForward(forward)
        if (result.valid) {
            reload()
            _message.value = "Forward saved"
        } else {
            _message.value = result.message ?: "Forward update failed"
        }
    }

    fun deleteForward(forwardId: String) {
        deps.configRepository.deleteForward(forwardId)
        reload()
        _message.value = "Forward deleted"
    }

    fun localhostUrl(forward: ForwardConfig): String = "http://${forward.localHost}:${forward.localPort}"
}

class LogsViewModel(private val deps: AppDependencies) : ViewModel() {
    private val _logs = MutableStateFlow<List<LogEvent>>(emptyList())
    val logs: StateFlow<List<LogEvent>> = _logs.asStateFlow()
    private val _filter = MutableStateFlow("all")
    val filter: StateFlow<String> = _filter.asStateFlow()
    private val _message = MutableStateFlow<String?>(null)
    val message: StateFlow<String?> = _message.asStateFlow()

    fun refresh(maxEvents: Int = 200) {
        _logs.value = deps.tunnelRepository.recentLogs(maxEvents)
    }

    fun setFilter(level: String) {
        _filter.value = level
    }

    fun filteredLogs(): List<LogEvent> {
        val selected = _filter.value
        return if (selected == "all") _logs.value else _logs.value.filter { it.level.equals(selected, ignoreCase = true) }
    }

    fun clearLogs() {
        _logs.value = emptyList()
    }

    fun exportDiagnostics(path: String, networkStatus: com.phillipchin.webrtctunnel.model.NetworkStatus) {
        deps.diagnosticsRepository.exportRedactedDiagnostics(
            outputPath = path,
            status = deps.tunnelRepository.status.value,
            logs = _logs.value,
            networkStatus = networkStatus,
        ).onSuccess {
            _message.value = "Diagnostics exported"
        }.onFailure {
            _message.value = it.message ?: "Diagnostics export failed"
        }
    }
}

class SettingsViewModel(private val deps: AppDependencies) : ViewModel() {
    fun validateConfig(): ValidationResult = deps.tunnelRepository.validateConfig(deps.configRepository.configPath)
}

class NetworkPolicyViewModel(private val deps: AppDependencies) : ViewModel() {
    val networkStatus = deps.networkPolicyManager.status
    val preferences = deps.configRepository.preferences

    fun savePreferences(updated: com.phillipchin.webrtctunnel.model.AndroidAppPreferences) {
        runBlocking { deps.configRepository.savePreferences(updated) }
    }
}

class ImportExportViewModel(private val deps: AppDependencies) : ViewModel() {
    private val _state = MutableStateFlow(ImportExportState())
    val state: StateFlow<ImportExportState> = _state.asStateFlow()

    fun updateState(transform: (ImportExportState) -> ImportExportState) {
        _state.value = transform(_state.value).copy(resultMessage = null)
    }

    fun importConfig() {
        val path = _state.value.configImportPath.trim()
        runCatching {
            val source = java.io.File(path)
            require(source.exists()) { "Config file not found" }
            deps.configRepository.writeConfig(source.readText())
            val validation = deps.tunnelRepository.validateConfig(deps.configRepository.configPath)
            require(validation.valid) { validation.message ?: "Config validation failed" }
        }.onSuccess {
            _state.value = _state.value.copy(resultMessage = "Config imported")
        }.onFailure {
            _state.value = _state.value.copy(resultMessage = it.message ?: "Config import failed")
        }
    }

    fun importPrivateIdentity() {
        deps.identityRepository.importPrivateIdentityFromPath(_state.value.privateIdentityImportPath.trim())
            .onSuccess { _state.value = _state.value.copy(resultMessage = "Private identity imported") }
            .onFailure { _state.value = _state.value.copy(resultMessage = it.message ?: "Private identity import failed") }
    }

    fun importPublicIdentity() {
        deps.identityRepository.appendAuthorizedPublicIdentity(_state.value.publicIdentityLine)
            .onSuccess { _state.value = _state.value.copy(resultMessage = "Public identity imported") }
            .onFailure { _state.value = _state.value.copy(resultMessage = it.message ?: "Public identity import failed") }
    }

    fun exportConfig() {
        runCatching {
            val output = java.io.File(_state.value.configExportPath.trim())
            output.parentFile?.mkdirs()
            output.writeText(deps.configRepository.readConfig())
        }.onSuccess {
            _state.value = _state.value.copy(resultMessage = "Config exported")
        }.onFailure {
            _state.value = _state.value.copy(resultMessage = it.message ?: "Config export failed")
        }
    }

    fun exportPublicIdentity() {
        deps.identityRepository.exportPublicIdentity(_state.value.publicIdentityExportPath.trim())
            .onSuccess { _state.value = _state.value.copy(resultMessage = "Public identity exported") }
            .onFailure { _state.value = _state.value.copy(resultMessage = it.message ?: "Public identity export failed") }
    }

    fun exportPrivateIdentity() {
        val current = _state.value
        deps.identityRepository.exportPrivateIdentity(
            outputPath = current.privateIdentityExportPath.trim(),
            confirmRisk = current.confirmPrivateExportRisk,
        ).onSuccess {
            _state.value = _state.value.copy(resultMessage = "Private identity exported")
        }.onFailure {
            _state.value = _state.value.copy(resultMessage = it.message ?: "Private identity export failed")
        }
    }
}

class AppViewModelFactory(private val deps: AppDependencies) {
    fun home() = HomeViewModel(deps)
    fun setup() = SetupViewModel(deps)
    fun forwards() = ForwardsViewModel(deps)
    fun logs() = LogsViewModel(deps)
    fun settings() = SettingsViewModel(deps)
    fun networkPolicy() = NetworkPolicyViewModel(deps)
    fun importExport() = ImportExportViewModel(deps)
}
