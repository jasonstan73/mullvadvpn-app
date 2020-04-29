package net.mullvad.mullvadvpn.service

import net.mullvad.talpid.ConnectivityListener

data class ServiceInstance(
    val daemon: MullvadDaemon,
    val connectionProxy: ConnectionProxy,
    val connectivityListener: ConnectivityListener,
    val locationInfoCache: LocationInfoCache
) {
    fun onDestroy() {
        connectionProxy.onDestroy()
        locationInfoCache.onDestroy()
    }
}
