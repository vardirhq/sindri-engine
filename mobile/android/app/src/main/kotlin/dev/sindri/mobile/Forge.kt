package dev.sindri.mobile

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

/**
 * The Forge palette exactly as the mobile prototype (`mobile/prototype/`) uses
 * it, which takes it from the desktop editor's theme (`editor/src/ui/theme.rs`):
 * graphite ground, warm metal for intent. Names say what a color is for, so a
 * screen is built from roles rather than hex values.
 */
object Forge {
    /** Behind everything: the app window and the canvas edge. */
    val Ground = Color(0xFF050608)
    /** Full-screen pages: the dashboard and New game. */
    val Page = Color(0xFF080A0D)
    /** Inputs, segmented controls and quiet cards. */
    val Well = Color(0xFF0B0E12)
    /** Sheets and cards. */
    val Sheet = Color(0xFF0F1317)
    /** A card lifted off a sheet. */
    val Card = Color(0xFF12161B)
    /** Icon wells, chips and raised buttons. */
    val Raised = Color(0xFF161B21)
    /** Dividers between list rows and around cards. */
    val Hairline = Color(0xFF1B2026)
    /** Borders of controls and panels. */
    val Line = Color(0xFF252B32)
    /** Disabled marks and empty dots. */
    val LineStrong = Color(0xFF3A424C)

    val Text = Color(0xFFE8E4DC)
    val TextMuted = Color(0xFF98A0A8)
    val TextFaint = Color(0xFF68717A)

    /** Primary actions, the selection, anything that means "do this". */
    val Metal = Color(0xFFE9A33A)
    val MetalBright = Color(0xFFFFBE55)
    /** The fill of a chosen chip, tab or row. */
    val MetalDeep = Color(0xFF2E2211)
    /** The border of a chosen chip, tab or row. */
    val MetalLine = Color(0xFF8A6220)
    /** Text and icons on a metal fill. */
    val OnMetal = Color(0xFF1A1206)

    val Success = Color(0xFF79B68A)
    val Danger = Color(0xFFFF8A94)
    val Info = Color(0xFF5B90D6)

    /** Menus, tips and toasts that float above everything. */
    val Popover = Color(0xFF1A1F26)

    /** Floating chrome over the canvas: 88.6 % ground, with a faint border. */
    val Glass = Color(0xE20B0E12)
    val GlassLine = Color(0xCC252B32)
    /** The selection chip's fill. */
    val SelectionGlass = Color(0xF52E2211)
    val Scrim = Color(0x80000000)

}

@Composable
fun ForgeTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = darkColorScheme(
            primary = Forge.Metal,
            onPrimary = Forge.OnMetal,
            background = Forge.Ground,
            onBackground = Forge.Text,
            surface = Forge.Sheet,
            onSurface = Forge.Text,
            error = Forge.Danger,
        ),
        typography = ForgeType.material,
        content = content,
    )
}
