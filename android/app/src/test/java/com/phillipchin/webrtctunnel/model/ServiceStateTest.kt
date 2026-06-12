package com.phillipchin.webrtctunnel.model

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ServiceStateTest {
    @Test
    fun listeningIsActiveAndRunning() {
        assertTrue(ServiceState.Listening.isTunnelActiveOrStarting())
        assertTrue(ServiceState.Listening.isTunnelRunning())
    }

    @Test
    fun connectedAndServingAreRunning() {
        assertTrue(ServiceState.Connected.isTunnelRunning())
        assertTrue(ServiceState.Serving.isTunnelRunning())
        assertTrue(ServiceState.Connected.isTunnelActiveOrStarting())
        assertTrue(ServiceState.Serving.isTunnelActiveOrStarting())
    }

    @Test
    fun startingConnectingReconnectingAreActiveButNotRunning() {
        listOf(ServiceState.Starting, ServiceState.Connecting, ServiceState.Reconnecting).forEach { state ->
            assertTrue("$state should be active-or-starting", state.isTunnelActiveOrStarting())
            assertFalse("$state should not be running", state.isTunnelRunning())
        }
    }

    @Test
    fun inactiveStatesAreNeitherActiveNorRunning() {
        listOf(
            ServiceState.Stopped,
            ServiceState.Stopping,
            ServiceState.Error,
            ServiceState.ConfigInvalid,
            ServiceState.PausedMeteredBlocked,
            ServiceState.NoNetwork,
        ).forEach { state ->
            assertFalse("$state should not be active-or-starting", state.isTunnelActiveOrStarting())
            assertFalse("$state should not be running", state.isTunnelRunning())
        }
    }
}
