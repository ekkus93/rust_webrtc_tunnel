package com.phillipchin.webrtctunnel.ui

import com.phillipchin.webrtctunnel.model.ForwardConfig
import org.junit.Assert.assertEquals
import org.junit.Test

class FlowScreensTest {
    @Test
    fun defaultNewForwardUsesSafeDefaults() {
        val existing = listOf(
            ForwardConfig(id = "a", name = "A", localHost = "127.0.0.1", localPort = 8080, remoteForwardId = "a", enabled = true),
            ForwardConfig(id = "b", name = "B", localHost = "127.0.0.1", localPort = 8081, remoteForwardId = "b", enabled = true),
        )

        val draft = defaultNewForward(existing)

        assertEquals("", draft.name)
        assertEquals("127.0.0.1", draft.localHost)
        assertEquals("", draft.remoteForwardId)
        assertEquals(8082, draft.localPort)
    }

    @Test
    fun suggestNewForwardPortSkipsDisabledEntries() {
        val existing = listOf(
            ForwardConfig(id = "a", name = "A", localHost = "127.0.0.1", localPort = 8080, remoteForwardId = "a", enabled = false),
            ForwardConfig(id = "b", name = "B", localHost = "127.0.0.1", localPort = 8081, remoteForwardId = "b", enabled = true),
        )

        val port = suggestNewForwardPort(existing, startPort = 8080)

        assertEquals(8080, port)
    }
}
