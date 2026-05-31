package com.phillipchin.webrtctunnel.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Terminal
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.navigation.NavHostController
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.phillipchin.webrtctunnel.data.AppDependencies
import com.phillipchin.webrtctunnel.ui.theme.WebRtcTunnelTheme
import com.phillipchin.webrtctunnel.viewmodel.AppViewModelFactory

private enum class ScreenTab(val route: String) { Home("home"), Forwards("forwards"), Logs("logs"), Settings("settings") }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WebRtcTunnelApp(deps: AppDependencies) {
    val factory = remember(deps) { AppViewModelFactory(deps) }
    val homeViewModel = remember { factory.home() }
    val setupViewModel = remember { factory.setup() }
    val forwardsViewModel = remember { factory.forwards() }
    val logsViewModel = remember { factory.logs() }
    val settingsViewModel = remember { factory.settings() }
    val navController = rememberNavController()

    WebRtcTunnelTheme {
        Scaffold(
            topBar = { TopAppBar(title = { Text("WebRTC Tunnel") }) },
            bottomBar = {
                BottomNavBar(navController)
            }
        ) { padding ->
            NavHost(navController = navController, startDestination = ScreenTab.Home.route) {
                composable(ScreenTab.Home.route) { HomeScreen(padding, homeViewModel) }
                composable(ScreenTab.Forwards.route) { ForwardsScreen(padding, forwardsViewModel) }
                composable(ScreenTab.Logs.route) { LogsScreen(padding, logsViewModel) }
                composable(ScreenTab.Settings.route) { SettingsScreen(padding, settingsViewModel, setupViewModel) }
            }
        }
    }
}

@Composable
private fun BottomNavBar(navController: NavHostController) {
    val currentRoute = navController.currentBackStackEntryAsState().value?.destination?.route
    NavigationBar {
        NavigationBarItem(selected = currentRoute == ScreenTab.Home.route, onClick = { navController.navigate(ScreenTab.Home.route) }, icon = { Icon(Icons.Default.Home, null) }, label = { Text("Home") })
        NavigationBarItem(selected = currentRoute == ScreenTab.Forwards.route, onClick = { navController.navigate(ScreenTab.Forwards.route) }, icon = { Icon(Icons.AutoMirrored.Filled.List, null) }, label = { Text("Forwards") })
        NavigationBarItem(selected = currentRoute == ScreenTab.Logs.route, onClick = { navController.navigate(ScreenTab.Logs.route) }, icon = { Icon(Icons.Default.Terminal, null) }, label = { Text("Logs") })
        NavigationBarItem(selected = currentRoute == ScreenTab.Settings.route, onClick = { navController.navigate(ScreenTab.Settings.route) }, icon = { Icon(Icons.Default.Settings, null) }, label = { Text("Settings") })
    }
}
