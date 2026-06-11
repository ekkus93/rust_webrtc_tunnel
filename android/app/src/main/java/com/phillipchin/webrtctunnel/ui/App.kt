package com.phillipchin.webrtctunnel.ui

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Terminal
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.ContextCompat
import androidx.navigation.NavDestination
import androidx.navigation.NavDestination.Companion.hierarchy
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.NavHostController
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import com.phillipchin.webrtctunnel.data.AppDependencies
import com.phillipchin.webrtctunnel.ui.theme.WebRtcTunnelTheme
import com.phillipchin.webrtctunnel.viewmodel.AppViewModelFactory
import com.phillipchin.webrtctunnel.viewmodel.ForwardsViewModel
import com.phillipchin.webrtctunnel.viewmodel.HomeViewModel
import com.phillipchin.webrtctunnel.viewmodel.ImportExportViewModel
import com.phillipchin.webrtctunnel.viewmodel.LogsViewModel
import com.phillipchin.webrtctunnel.viewmodel.NetworkPolicyViewModel
import com.phillipchin.webrtctunnel.viewmodel.SettingsViewModel
import com.phillipchin.webrtctunnel.viewmodel.SetupViewModel

private sealed class Route(val value: String, val title: String) {
    data object Home : Route("home", "WebRTC Tunnel")

    data object Forwards : Route("forwards", "Forwards")

    data object Logs : Route("logs", "Logs")

    data object Settings : Route("settings", "Settings")

    data object Setup : Route("setup", "Setup Wizard")

    data object NetworkPolicy : Route("network_policy", "Network Policy")

    data object ImportExport : Route("import_export", "Import / Export")

    data object ForwardDetails : Route("forwardDetails/{forwardId}", "Forward Details")
}

private data class BottomTab(
    val route: Route,
    val label: String,
    val icon: @Composable () -> Unit,
)

private val mainTabs =
    listOf(
        BottomTab(Route.Home, "Home", { Icon(Icons.Default.Home, "Home tab icon") }),
        BottomTab(Route.Forwards, "Forwards", { Icon(Icons.AutoMirrored.Filled.List, "Forwards tab icon") }),
        BottomTab(Route.Logs, "Logs", { Icon(Icons.Default.Terminal, "Logs tab icon") }),
        BottomTab(Route.Settings, "Settings", { Icon(Icons.Default.Settings, "Settings tab icon") }),
    )

private val secondaryRoutes =
    setOf(
        Route.Setup.value,
        Route.NetworkPolicy.value,
        Route.ImportExport.value,
        "forwardDetails/{forwardId}",
    )

@Composable
fun WebRtcTunnelApp(deps: AppDependencies) {
    val factory = remember(deps) { AppViewModelFactory(deps) }
    val models = remember(factory) { AppScreenModels(factory) }
    val navController = rememberNavController()

    WebRtcTunnelTheme {
        NotificationPermissionGate()
        val backStackEntry by navController.currentBackStackEntryAsState()
        val currentRoute = backStackEntry?.destination?.route
        val showBottomBar = currentRoute in mainTabs.map { it.route.value }
        val showBackArrow = currentRoute != null && currentRoute in secondaryRoutes
        val title = routeTitle(currentRoute)

        Scaffold(
            topBar = {
                TunnelTopAppBar(
                    title = title,
                    navigationIcon =
                        if (showBackArrow) {
                            (
                                {
                                    IconButton(onClick = { navController.navigateUp() }) {
                                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Back")
                                    }
                                }
                            )
                        } else {
                            null
                        },
                )
            },
            bottomBar = {
                if (showBottomBar) {
                    BottomNavBar(navController)
                }
            },
        ) { padding ->
            AppNavHost(navController = navController, padding = padding, models = models)
        }
    }
}

private class AppScreenModels(factory: AppViewModelFactory) {
    val home: HomeViewModel = factory.home()
    val setup: SetupViewModel = factory.setup()
    val forwards: ForwardsViewModel = factory.forwards()
    val logs: LogsViewModel = factory.logs()
    val settings: SettingsViewModel = factory.settings()
    val networkPolicy: NetworkPolicyViewModel = factory.networkPolicy()
    val importExport: ImportExportViewModel = factory.importExport()
}

private fun homeNavActions(navController: NavHostController) =
    HomeNavActions(
        onOpenSetup = { navController.navigate(Route.Setup.value) },
        onOpenLogs = { navController.navigate(Route.Logs.value) },
        onOpenSettings = { navController.navigate(Route.Settings.value) },
        onOpenForwardDetails = { id -> navController.navigate("forwardDetails/$id") },
    )

private fun settingsNavActions(navController: NavHostController) =
    SettingsNavActions(
        onOpenSetup = { navController.navigate(Route.Setup.value) },
        onOpenLogs = { navController.navigate(Route.Logs.value) },
        onOpenNetworkPolicy = { navController.navigate(Route.NetworkPolicy.value) },
        onOpenImportExport = { navController.navigate(Route.ImportExport.value) },
    )

@Composable
private fun AppNavHost(
    navController: NavHostController,
    padding: PaddingValues,
    models: AppScreenModels,
) {
    NavHost(navController = navController, startDestination = Route.Home.value) {
        composable(Route.Home.value) {
            HomeScreen(
                padding = padding,
                vm = models.home,
                forwardsVm = models.forwards,
                nav = homeNavActions(navController),
            )
        }
        composable(Route.Forwards.value) {
            ForwardsScreen(
                padding = padding,
                vm = models.forwards,
                onOpenDetails = { forwardId ->
                    navController.navigate("forwardDetails/$forwardId")
                },
            )
        }
        composable(Route.Logs.value) { LogsScreen(padding, models.logs, models.networkPolicy) }
        composable(Route.Settings.value) {
            SettingsScreen(padding = padding, vm = models.settings, nav = settingsNavActions(navController))
        }
        composable(Route.Setup.value) {
            SetupWizardScreen(
                padding = padding,
                vm = models.setup,
                onStartSuccess = {
                    navController.navigate(Route.Home.value) {
                        popUpTo(Route.Home.value) { inclusive = false }
                        launchSingleTop = true
                    }
                },
            )
        }
        composable(Route.NetworkPolicy.value) { NetworkPolicyScreen(padding, models.networkPolicy) }
        composable(Route.ImportExport.value) { ImportExportScreen(padding, models.importExport) }
        composable(
            route = Route.ForwardDetails.value,
            arguments = listOf(navArgument("forwardId") { type = NavType.StringType }),
        ) { backStack ->
            ForwardDetailsScreen(
                padding = padding,
                vm = models.forwards,
                forwardId = backStack.arguments?.getString("forwardId").orEmpty(),
                onDeleteAndReturn = { navController.navigateUp() },
            )
        }
    }
}

@Composable
private fun NotificationPermissionGate() {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
    val context = LocalContext.current
    val hasPermission =
        ContextCompat.checkSelfPermission(
            context,
            Manifest.permission.POST_NOTIFICATIONS,
        ) == PackageManager.PERMISSION_GRANTED
    if (hasPermission) return

    var openDialog by remember { mutableStateOf(true) }
    var denied by remember { mutableStateOf(false) }
    val launcher =
        rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
            denied = !granted
            openDialog = !granted
        }
    if (openDialog) {
        NotificationRequestDialog(
            onAllow = { launcher.launch(Manifest.permission.POST_NOTIFICATIONS) },
            onNotNow = {
                denied = true
                openDialog = false
            },
            onDismiss = { openDialog = false },
        )
    }
    if (denied) {
        NotificationsDisabledDialog(
            onOpenAppSettings = {
                val intent =
                    Intent(
                        Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                        Uri.fromParts("package", context.packageName, null),
                    ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                context.startActivity(intent)
                denied = false
            },
            onClose = { denied = false },
        )
    }
}

@Composable
private fun NotificationRequestDialog(
    onAllow: () -> Unit,
    onNotNow: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Notification permission") },
        text = {
            Text(
                "Rust WebRTC Tunnel needs notifications so Android can keep the tunnel " +
                    "service visible while it is running in the background.",
            )
        },
        confirmButton = { TextButton(onClick = onAllow) { Text("Allow") } },
        dismissButton = { TextButton(onClick = onNotNow) { Text("Not now") } },
    )
}

@Composable
private fun NotificationsDisabledDialog(
    onOpenAppSettings: () -> Unit,
    onClose: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onClose,
        title = { Text("Notifications are disabled") },
        text = { Text("Background tunnel notifications are required for full foreground-service visibility.") },
        confirmButton = { TextButton(onClick = onOpenAppSettings) { Text("Open Settings") } },
        dismissButton = { TextButton(onClick = onClose) { Text("Close") } },
    )
}

@Composable
private fun BottomNavBar(navController: NavHostController) {
    val navBackStackEntry by navController.currentBackStackEntryAsState()
    val currentDestination = navBackStackEntry?.destination
    NavigationBar {
        mainTabs.forEach { tab ->
            NavigationBarItem(
                selected = currentDestination.isOnRoute(tab.route.value),
                onClick = {
                    navController.navigate(tab.route.value) {
                        popUpTo(navController.graph.findStartDestination().id) {
                            saveState = true
                        }
                        launchSingleTop = true
                        restoreState = true
                    }
                },
                icon = tab.icon,
                label = { Text(tab.label) },
            )
        }
    }
}

private fun NavDestination?.isOnRoute(route: String): Boolean {
    return this?.hierarchy?.any { it.route == route } == true
}

private fun routeTitle(route: String?): String =
    when {
        route == Route.Home.value -> Route.Home.title
        route == Route.Forwards.value -> Route.Forwards.title
        route == Route.Logs.value -> Route.Logs.title
        route == Route.Settings.value -> Route.Settings.title
        route == Route.Setup.value -> Route.Setup.title
        route == Route.NetworkPolicy.value -> Route.NetworkPolicy.title
        route == Route.ImportExport.value -> Route.ImportExport.title
        route?.startsWith("forwardDetails/") == true -> Route.ForwardDetails.title
        else -> "WebRTC Tunnel"
    }
