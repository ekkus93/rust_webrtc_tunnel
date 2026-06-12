package com.phillipchin.webrtctunnel.data

import com.phillipchin.webrtctunnel.model.ForwardConfig
import com.phillipchin.webrtctunnel.model.ValidationResult
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext

/**
 * Single source of truth for the configured local forwards. Wraps [ForwardsConfigStore]
 * persistence behind an observable [StateFlow] so Home and Forwards screens stay in sync
 * after any edit, and performs all disk IO on the injected IO dispatcher.
 *
 * On a corrupt forwards file [refresh] keeps the current in-memory list rather than
 * erasing it (the store logs the corruption).
 */
class ForwardsRepository(
    private val store: ForwardsConfigStore,
    private val dispatchers: AppDispatchers,
) {
    private val _forwards = MutableStateFlow(store.loadForwards())
    val forwards: StateFlow<List<ForwardConfig>> = _forwards.asStateFlow()

    fun current(): List<ForwardConfig> = _forwards.value

    suspend fun refresh() {
        withContext(dispatchers.io) {
            store.loadForwardsResult().onSuccess { _forwards.value = it }
            // onFailure: keep the existing in-memory list (store already logged).
        }
    }

    suspend fun upsert(forward: ForwardConfig): ValidationResult =
        withContext(dispatchers.io) {
            val result = store.upsertForward(forward)
            _forwards.value = store.loadForwards()
            result
        }

    suspend fun delete(forwardId: String) {
        withContext(dispatchers.io) {
            store.deleteForward(forwardId)
            _forwards.value = store.loadForwards()
        }
    }

    suspend fun save(forwards: List<ForwardConfig>) {
        withContext(dispatchers.io) {
            store.saveForwards(forwards)
            _forwards.value = forwards
        }
    }
}
