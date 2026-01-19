package com.privmsg.app.ui.theme

import android.app.Activity
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.SideEffect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat

// Telegram iOS-like colors
val PrimaryBlue = Color(0xFF007AFF)
val SecondaryBlue = Color(0xFF5AC8FA)
val Green = Color(0xFF34C759)
val Red = Color(0xFFFF3B30)
val Orange = Color(0xFFFF9500)

// Light theme
val LightBackground = Color(0xFFFFFFFF)
val LightSurface = Color(0xFFF2F2F7)
val LightOnSurface = Color(0xFF000000)
val LightSecondaryText = Color(0xFF8E8E93)
val LightDivider = Color(0xFFC6C6C8)

// Dark theme
val DarkBackground = Color(0xFF000000)
val DarkSurface = Color(0xFF1C1C1E)
val DarkOnSurface = Color(0xFFFFFFFF)
val DarkSecondaryText = Color(0xFF8E8E93)
val DarkDivider = Color(0xFF38383A)

// Message bubbles
val OutgoingBubble = PrimaryBlue
val IncomingBubbleLight = Color(0xFFE9E9EB)
val IncomingBubbleDark = Color(0xFF262628)

private val LightColorScheme = lightColorScheme(
    primary = PrimaryBlue,
    onPrimary = Color.White,
    secondary = SecondaryBlue,
    onSecondary = Color.White,
    background = LightBackground,
    onBackground = LightOnSurface,
    surface = LightSurface,
    onSurface = LightOnSurface,
    error = Red,
    onError = Color.White,
    outline = LightDivider,
    surfaceVariant = LightSurface,
    onSurfaceVariant = LightSecondaryText,
)

private val DarkColorScheme = darkColorScheme(
    primary = PrimaryBlue,
    onPrimary = Color.White,
    secondary = SecondaryBlue,
    onSecondary = Color.White,
    background = DarkBackground,
    onBackground = DarkOnSurface,
    surface = DarkSurface,
    onSurface = DarkOnSurface,
    error = Red,
    onError = Color.White,
    outline = DarkDivider,
    surfaceVariant = DarkSurface,
    onSurfaceVariant = DarkSecondaryText,
)

@Composable
fun PrivMsgTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit
) {
    val colorScheme = if (darkTheme) DarkColorScheme else LightColorScheme

    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as Activity).window
            window.statusBarColor = colorScheme.background.toArgb()
            WindowCompat.getInsetsController(window, view).isAppearanceLightStatusBars = !darkTheme
        }
    }

    MaterialTheme(
        colorScheme = colorScheme,
        typography = Typography,
        content = content
    )
}
