package com.phillipchin.webrtctunnel

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import com.phillipchin.webrtctunnel.data.AppDependencies
import com.phillipchin.webrtctunnel.ui.WebRtcTunnelApp

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val deps = AppDependencies(applicationContext)
        deps.configRepository.ensureDefaultConfig(deps.configRepository.defaultConfigTemplate())
        setContent {
            WebRtcTunnelApp(deps)
        }
    }
}
