package com.phillipchin.webrtctunnel.data

import android.content.Context
import com.phillipchin.webrtctunnel.network.NetworkPolicyManager
import com.phillipchin.webrtctunnel.security.IdentityRepository

class AppDependencies(context: Context) {
    val configRepository = ConfigRepository(context)
    val tunnelRepository = TunnelRepository(context)
    val networkPolicyManager = NetworkPolicyManager(context)
    val identityRepository = IdentityRepository(context)
}
