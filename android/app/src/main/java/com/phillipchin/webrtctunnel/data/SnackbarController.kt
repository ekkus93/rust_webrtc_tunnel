package com.phillipchin.webrtctunnel.data

import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow

/**
 * App-wide one-shot user messages surfaced as snackbars. Backed by a [SharedFlow] with no
 * replay, so a message is shown exactly once and never re-appears on recomposition or when
 * navigating back to a screen (unlike a `StateFlow`, which retains its last value). Emitting
 * is non-suspending and drops the oldest buffered message under burst, so callers in any
 * context can fire-and-forget.
 */
class SnackbarController {
    private val _messages =
        MutableSharedFlow<String>(extraBufferCapacity = 8, onBufferOverflow = BufferOverflow.DROP_OLDEST)
    val messages: SharedFlow<String> = _messages.asSharedFlow()

    fun show(message: String) {
        _messages.tryEmit(message)
    }
}
