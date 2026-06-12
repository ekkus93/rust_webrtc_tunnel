package com.phillipchin.webrtctunnel.data

import android.content.Context
import android.util.Log
import com.phillipchin.webrtctunnel.model.ForwardConfig
import com.phillipchin.webrtctunnel.model.ValidationResult
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File
import java.nio.file.AtomicMoveNotSupportedException
import java.nio.file.Files
import java.nio.file.StandardCopyOption

private const val MAX_PORT = 65535
private const val TAG = "ForwardsConfigStore"

/**
 * Low-level persistence + validation for the configured local forwards. Mutation /
 * in-memory source-of-truth lives in [ForwardsRepository]; this class only loads,
 * saves (atomically), and validates.
 */
class ForwardsConfigStore(private val context: Context) {
    private val forwardsFile: File get() = File(context.filesDir, "forwards.json")

    private fun defaultForwards(): List<ForwardConfig> =
        listOf(
            ForwardConfig(
                id = "llama",
                name = "Llama server",
                localHost = "127.0.0.1",
                localPort = 8080,
                remoteForwardId = "llama",
                enabled = true,
            ),
        )

    /**
     * Load forwards, distinguishing a corrupt file (failure) from a legitimately
     * empty/missing one (success). On a missing file the defaults are seeded and
     * returned; on corrupt JSON the error is logged and surfaced so callers can keep
     * their existing in-memory list rather than silently erasing it.
     */
    fun loadForwardsResult(): Result<List<ForwardConfig>> {
        if (!forwardsFile.exists()) {
            val defaults = defaultForwards()
            saveForwards(defaults)
            return Result.success(defaults)
        }
        return runCatching { Json.decodeFromString<List<ForwardConfig>>(forwardsFile.readText()) }
            .onFailure { error ->
                Log.w(TAG, "forwards.json is corrupt; keeping existing forwards instead of erasing", error)
            }
    }

    /** Read-only convenience for display paths. Mutation paths must use
     * [loadForwardsResult] (or [ForwardsRepository]) so corruption is never treated as empty. */
    fun loadForwards(): List<ForwardConfig> = loadForwardsResult().getOrElse { emptyList() }

    /**
     * Atomically replace forwards.json: write a temp file in the same directory and
     * move it into place (atomic when the filesystem supports it, replace otherwise).
     * Never direct-writes the destination, so a crash mid-write cannot truncate it.
     */
    fun saveForwards(forwards: List<ForwardConfig>) {
        val dir = forwardsFile.parentFile
        dir?.mkdirs()
        val temp = File.createTempFile("forwards", ".json.tmp", dir)
        try {
            temp.writeText(Json.encodeToString(forwards))
            try {
                Files.move(
                    temp.toPath(),
                    forwardsFile.toPath(),
                    StandardCopyOption.ATOMIC_MOVE,
                    StandardCopyOption.REPLACE_EXISTING,
                )
            } catch (_: AtomicMoveNotSupportedException) {
                Files.move(temp.toPath(), forwardsFile.toPath(), StandardCopyOption.REPLACE_EXISTING)
            }
        } finally {
            temp.delete()
        }
    }

    /**
     * Disk-based upsert used by the setup wizard. Corrupt-safe: a corrupt existing file
     * is reported as a failure rather than treated as empty and overwritten.
     * (Home/Forwards mutate through [ForwardsRepository] on the in-memory list instead.)
     */
    fun upsertForward(forward: ForwardConfig): ValidationResult {
        val existing =
            loadForwardsResult().getOrElse {
                return ValidationResult(false, "Saved forwards file is corrupt; not overwriting")
            }
        val updated =
            existing.toMutableList().apply {
                val index = indexOfFirst { it.id == forward.id }
                if (index >= 0) set(index, forward) else add(forward)
            }
        val error = validateForwards(updated)
        return if (error != null) {
            ValidationResult(false, error)
        } else {
            saveForwards(updated)
            ValidationResult(true, null)
        }
    }

    /** Disk-based delete used by the setup wizard. Corrupt-safe: a corrupt file is left
     * untouched rather than overwritten with a list that dropped the corrupt entries. */
    fun deleteForward(forwardId: String) {
        val existing = loadForwardsResult().getOrNull() ?: return
        saveForwards(existing.filterNot { it.id == forwardId })
    }

    fun validateForwards(forwards: List<ForwardConfig>): String? {
        val duplicateId = forwards.groupBy { it.id }.entries.firstOrNull { it.value.size > 1 }?.key
        val enabled = forwards.filter { it.enabled }
        val duplicatePort = enabled.groupBy { it.localPort }.entries.firstOrNull { it.value.size > 1 }?.key
        val duplicateRemoteForwardId =
            enabled
                .groupBy { it.remoteForwardId.trim() }
                .entries
                .firstOrNull { it.key.isNotBlank() && it.value.size > 1 }
                ?.key
        return when {
            duplicateId != null -> "Duplicate forward id: $duplicateId"
            enabled.any { it.name.trim().isBlank() } -> "Forward name is required"
            duplicatePort != null -> "Duplicate local port: $duplicatePort"
            duplicateRemoteForwardId != null -> "Duplicate remote forward ID: $duplicateRemoteForwardId"
            enabled.any { it.remoteForwardId.isBlank() } -> "Remote forward ID is required"
            enabled.any { it.localPort !in 1..MAX_PORT } -> "Port must be between 1 and 65535"
            enabled.any { it.localHost != "127.0.0.1" && it.localHost != "localhost" } ->
                "Non-localhost bind requires advanced warning"
            else -> null
        }
    }
}
