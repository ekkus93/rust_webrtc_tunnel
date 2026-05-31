package com.phillipchin.webrtctunnel

import android.app.Application
import com.phillipchin.webrtctunnel.data.AppDependencies
import com.phillipchin.webrtctunnel.notification.NotificationController

class WebRtcTunnelApplication : Application() {
    lateinit var deps: AppDependencies
        private set

    override fun onCreate() {
        super.onCreate()
        deps = AppDependencies(this)
        NotificationController(this).ensureChannels()
        deps.configRepository.ensureDefaultConfig(deps.configRepository.defaultConfigTemplate())
    }
}
