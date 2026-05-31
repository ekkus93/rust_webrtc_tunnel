package com.phillipchin.webrtctunnel.data

import android.content.Context
import com.phillipchin.webrtctunnel.network.NetworkPolicyManager
import com.phillipchin.webrtctunnel.security.IdentityRepository

class AppDependencies(context: Context) {
    private val appContext = context.applicationContext
    val configRepository = ConfigRepository(appContext)
    val tunnelRepository = TunnelRepository(appContext)
    val networkPolicyManager = NetworkPolicyManager(appContext)
    val identityRepository = IdentityRepository(appContext)
    val context: Context = appContext
}
