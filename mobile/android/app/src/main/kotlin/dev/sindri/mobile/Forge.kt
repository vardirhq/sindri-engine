package dev.sindri.mobile

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

/**
 * The Forge palette, as the mobile prototype uses it (`mobile/prototype/`),
 * which takes it from the desktop editor's theme (`editor/src/ui/theme.rs`):
 * graphite ground, warm metal for intent.
 */
object Forge {
    val Ground = Color(0xFF050608)
    val Panel = Color(0xE6161B21)
    val PanelRaised = Color(0xFF1B2026)
    val Line = Color(0xFF252B32)
    val Text = Color(0xFFE8E4DC)
    val TextMuted = Color(0xFF98A0A8)
    val TextFaint = Color(0xFF68717A)
    val Metal = Color(0xFFE9A33A)
    val MetalBright = Color(0xFFFFBE55)
    val MetalDeep = Color(0xFF2E2211)
    val Danger = Color(0xFFE5675C)
}

@Composable
fun ForgeTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = darkColorScheme(
            primary = Forge.Metal,
            onPrimary = Forge.Ground,
            background = Forge.Ground,
            onBackground = Forge.Text,
            surface = Forge.PanelRaised,
            onSurface = Forge.Text,
            error = Forge.Danger,
        ),
        content = content,
    )
}
