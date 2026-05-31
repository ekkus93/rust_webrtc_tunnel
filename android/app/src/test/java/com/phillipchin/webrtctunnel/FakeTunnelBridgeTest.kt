package com.phillipchin.webrtctunnel

import com.phillipchin.webrtctunnel.model.LogEvent
import com.phillipchin.webrtctunnel.model.ServiceState
import com.phillipchin.webrtctunnel.model.TunnelMode
import com.phillipchin.webrtctunnel.model.TunnelStatus
import kotlinx.serialization.json.Json
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class FakeTunnelBridgeTest {
    private val json = Json { ignoreUnknownKeys = true }

    @Test
    fun fakeBridgeReturnsStatusJson() {
        val bridge = FakeTunnelBridge()
        val status = json.decodeFromString(TunnelStatus.serializer(), bridge.getStatusJson())
        assertEquals(ServiceState.Stopped, status.serviceState)
        assertEquals(TunnelMode.Offer, status.mode)
    }

    @Test
    fun fakeBridgeReturnsLogsJson() {
        val bridge = FakeTunnelBridge()
        val logs = json.decodeFromString<List<LogEvent>>(bridge.getRecentLogsJson(2))
        assertTrue(logs.isNotEmpty())
    }
}
