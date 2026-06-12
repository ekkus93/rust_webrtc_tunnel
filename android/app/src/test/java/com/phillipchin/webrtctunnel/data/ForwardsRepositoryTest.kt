package com.phillipchin.webrtctunnel.data

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import com.phillipchin.webrtctunnel.model.ForwardConfig
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.File

@RunWith(RobolectricTestRunner::class)
class ForwardsRepositoryTest {
    private val context = ApplicationProvider.getApplicationContext<Context>()
    private val file get() = File(context.filesDir, "forwards.json")
    private lateinit var repo: ForwardsRepository

    @Before
    fun setUp() {
        file.delete()
        // Real dispatchers; suspend repository methods complete under runBlocking.
        repo = ForwardsRepository(ForwardsConfigStore(context), AppDispatchers())
    }

    private fun forward(
        id: String,
        port: Int,
    ) = ForwardConfig(id = id, name = id, localPort = port, remoteForwardId = id, enabled = true)

    @Test
    fun upsertUpdatesObservableState() =
        runBlocking {
            val result = repo.upsert(forward("x", 1234))
            assertTrue(result.valid)
            assertTrue(repo.forwards.value.any { it.id == "x" })
        }

    @Test
    fun deleteUpdatesObservableState() =
        runBlocking {
            repo.save(listOf(forward("d", 3333)))
            assertTrue(repo.forwards.value.any { it.id == "d" })
            repo.delete("d")
            assertTrue(repo.forwards.value.none { it.id == "d" })
        }

    @Test
    fun refreshKeepsPriorListWhenFileIsCorrupt() =
        runBlocking {
            repo.save(listOf(forward("keep", 2222)))
            assertTrue(repo.forwards.value.any { it.id == "keep" })

            file.writeText("{ corrupt json")
            repo.refresh()

            // The corrupt file must not erase the in-memory list.
            assertTrue(repo.forwards.value.any { it.id == "keep" })
        }
}
