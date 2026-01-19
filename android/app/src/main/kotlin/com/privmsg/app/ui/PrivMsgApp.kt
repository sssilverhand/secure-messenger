package com.privmsg.app.ui

import androidx.compose.runtime.*
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import com.privmsg.app.ui.screens.*

sealed class Screen(val route: String) {
    object Splash : Screen("splash")
    object Login : Screen("login")
    object Home : Screen("home")
    object Chat : Screen("chat/{userId}") {
        fun createRoute(userId: String) = "chat/$userId"
    }
    object NewChat : Screen("new_chat")
    object Settings : Screen("settings")
    object Call : Screen("call/{userId}?video={video}&incoming={incoming}") {
        fun createRoute(userId: String, video: Boolean = false, incoming: Boolean = false) =
            "call/$userId?video=$video&incoming=$incoming"
    }
}

@Composable
fun PrivMsgApp() {
    val navController = rememberNavController()

    NavHost(
        navController = navController,
        startDestination = Screen.Splash.route
    ) {
        composable(Screen.Splash.route) {
            SplashScreen(
                onNavigateToLogin = {
                    navController.navigate(Screen.Login.route) {
                        popUpTo(Screen.Splash.route) { inclusive = true }
                    }
                },
                onNavigateToHome = {
                    navController.navigate(Screen.Home.route) {
                        popUpTo(Screen.Splash.route) { inclusive = true }
                    }
                }
            )
        }

        composable(Screen.Login.route) {
            LoginScreen(
                onLoginSuccess = {
                    navController.navigate(Screen.Home.route) {
                        popUpTo(Screen.Login.route) { inclusive = true }
                    }
                }
            )
        }

        composable(Screen.Home.route) {
            HomeScreen(
                onChatClick = { userId ->
                    navController.navigate(Screen.Chat.createRoute(userId))
                },
                onNewChatClick = {
                    navController.navigate(Screen.NewChat.route)
                },
                onSettingsClick = {
                    navController.navigate(Screen.Settings.route)
                }
            )
        }

        composable(
            route = Screen.Chat.route,
            arguments = listOf(navArgument("userId") { type = NavType.StringType })
        ) { backStackEntry ->
            val userId = backStackEntry.arguments?.getString("userId") ?: return@composable
            ChatScreen(
                userId = userId,
                onBack = { navController.popBackStack() },
                onCallClick = { isVideo ->
                    navController.navigate(Screen.Call.createRoute(userId, isVideo))
                }
            )
        }

        composable(Screen.NewChat.route) {
            NewChatScreen(
                onBack = { navController.popBackStack() },
                onStartChat = { userId ->
                    navController.navigate(Screen.Chat.createRoute(userId)) {
                        popUpTo(Screen.NewChat.route) { inclusive = true }
                    }
                }
            )
        }

        composable(Screen.Settings.route) {
            SettingsScreen(
                onBack = { navController.popBackStack() },
                onLogout = {
                    navController.navigate(Screen.Login.route) {
                        popUpTo(Screen.Home.route) { inclusive = true }
                    }
                }
            )
        }

        composable(
            route = Screen.Call.route,
            arguments = listOf(
                navArgument("userId") { type = NavType.StringType },
                navArgument("video") { type = NavType.BoolType; defaultValue = false },
                navArgument("incoming") { type = NavType.BoolType; defaultValue = false }
            )
        ) { backStackEntry ->
            val userId = backStackEntry.arguments?.getString("userId") ?: return@composable
            val isVideo = backStackEntry.arguments?.getBoolean("video") ?: false
            val isIncoming = backStackEntry.arguments?.getBoolean("incoming") ?: false
            CallScreen(
                userId = userId,
                isVideo = isVideo,
                isIncoming = isIncoming,
                onEnd = { navController.popBackStack() }
            )
        }
    }
}
