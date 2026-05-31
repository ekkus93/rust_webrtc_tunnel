package com.phillipchin.webrtctunnel.network

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import com.phillipchin.webrtctunnel.model.NetworkStatus
import com.phillipchin.webrtctunnel.model.NetworkType
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class NetworkPolicyManager(private val context: Context) {
    private val _status = MutableStateFlow(classifyCurrentNetwork())
    val status: StateFlow<NetworkStatus> = _status

    fun refresh() {
        _status.value = classifyCurrentNetwork()
    }

    fun allowTunnelOnCurrentNetwork(allowMetered: Boolean): Boolean {
        val status = classifyCurrentNetwork()
        return when (status.networkType) {
            NetworkType.NoNetwork, NetworkType.Unknown -> false
            NetworkType.Cellular, NetworkType.MeteredWifi -> allowMetered
            NetworkType.UnmeteredWifi -> true
        }
    }

    private fun classifyCurrentNetwork(): NetworkStatus {
        val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val network = cm.activeNetwork ?: return NetworkStatus(NetworkType.NoNetwork, false, false, "No network")
        val capabilities = cm.getNetworkCapabilities(network) ?: return NetworkStatus(NetworkType.Unknown, false, false, "Unknown network")
        val metered = cm.isActiveNetworkMetered
        val networkType = when {
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) && !metered -> NetworkType.UnmeteredWifi
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) -> NetworkType.MeteredWifi
            capabilities.hasTransport(NetworkCapabilities.TRANSPORT_CELLULAR) -> NetworkType.Cellular
            else -> NetworkType.Unknown
        }
        val allowed = networkType == NetworkType.UnmeteredWifi
        val reason = if (allowed) null else "Tunnel blocked by policy"
        return NetworkStatus(networkType, metered, allowed, reason)
    }
}
