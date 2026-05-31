package com.phillipchin.webrtctunnel

import com.phillipchin.webrtctunnel.model.LogEvent
import com.phillipchin.webrtctunnel.model.ValidationResult
import com.phillipchin.webrtctunnel.model.TunnelStatus
import kotlinx.serialization.json.Json

interface TunnelNativeBridge {
    fun startOffer(configPath: String): Result<Unit>
    fun startAnswer(configPath: String): Result<Unit>
    fun stop(): Result<Unit>
    fun getStatusJson(): String
    fun getRecentLogsJson(maxEvents: Int): String
    fun validateConfig(configPath: String): ValidationResult
}

class RustTunnelBridge : TunnelNativeBridge {
    companion object {
        init {
            runCatching { System.loadLibrary("p2p_mobile") }
        }
    }

    private var runtimeHandle: Long = nativeCreateRuntime()

    override fun startOffer(configPath: String): Result<Unit> = runCatching {
        check(nativeStartOffer(runtimeHandle, configPath) == 0) { nativeLastError(runtimeHandle) }
    }

    override fun startAnswer(configPath: String): Result<Unit> = runCatching {
        check(nativeStartAnswer(runtimeHandle, configPath) == 0) { nativeLastError(runtimeHandle) }
    }

    override fun stop(): Result<Unit> = runCatching {
        check(nativeStop(runtimeHandle) == 0) { nativeLastError(runtimeHandle) }
    }

    override fun getStatusJson(): String = nativeStatusJson(runtimeHandle)

    override fun getRecentLogsJson(maxEvents: Int): String = nativeRecentLogsJson(runtimeHandle, maxEvents)

    override fun validateConfig(configPath: String): ValidationResult =
        Json.decodeFromString(nativeValidateConfig(configPath))

    fun dispose() {
        nativeDestroyRuntime(runtimeHandle)
        runtimeHandle = 0L
    }

    private external fun nativeCreateRuntime(): Long
    private external fun nativeDestroyRuntime(handle: Long)
    private external fun nativeStartOffer(handle: Long, configPath: String): Int
    private external fun nativeStartAnswer(handle: Long, configPath: String): Int
    private external fun nativeStop(handle: Long): Int
    private external fun nativeStatusJson(handle: Long): String
    private external fun nativeRecentLogsJson(handle: Long, maxEvents: Int): String
    private external fun nativeValidateConfig(configPath: String): String
    private external fun nativeLastError(handle: Long): String
}

class FakeTunnelBridge : TunnelNativeBridge {
    private var status = TunnelStatus(
        serviceState = com.phillipchin.webrtctunnel.model.ServiceState.Stopped,
        mode = com.phillipchin.webrtctunnel.model.TunnelMode.Offer,
        localPeerId = "android-phone",
    )

    override fun startOffer(configPath: String): Result<Unit> = runCatching {
        status = status.copy(serviceState = com.phillipchin.webrtctunnel.model.ServiceState.Connected)
    }

    override fun startAnswer(configPath: String): Result<Unit> = runCatching {
        status = status.copy(serviceState = com.phillipchin.webrtctunnel.model.ServiceState.Serving)
    }

    override fun stop(): Result<Unit> = runCatching {
        status = status.copy(serviceState = com.phillipchin.webrtctunnel.model.ServiceState.Stopped)
    }

    override fun getStatusJson(): String = Json.encodeToString(TunnelStatus.serializer(), status)

    override fun getRecentLogsJson(maxEvents: Int): String =
        Json.encodeToString<List<LogEvent>>(
            List(maxEvents.coerceAtMost(3)) { LogEvent(0L, "info", "fake log $it") }
        )

    override fun validateConfig(configPath: String): ValidationResult = ValidationResult(true, null)
}
