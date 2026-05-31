package com.phillipchin.webrtctunnel

import android.app.Application
import com.phillipchin.webrtctunnel.notification.NotificationController

class WebRtcTunnelApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        NotificationController(this).ensureChannels()
    }
}
