package dev.sindri.mobile

import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.material3.Typography
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.ExperimentalTextApi
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * The prototype's three typefaces: Inter for everything, JetBrains Mono for
 * file names and Decay, and Material Symbols Outlined for icons, all bundled
 * (see `mobile/android/licenses/`) so the app looks the same offline.
 */
object ForgeType {
    // Inter is one variable font; each weight is a point on its axis.
    @OptIn(ExperimentalTextApi::class)
    private fun inter(weight: FontWeight) = Font(
        R.font.inter,
        weight = weight,
        variationSettings = FontVariation.Settings(FontVariation.weight(weight.weight)),
    )

    val Inter = FontFamily(
        inter(FontWeight.Normal),
        inter(FontWeight.Medium),
        inter(FontWeight.SemiBold),
        inter(FontWeight.Bold),
    )

    val Mono = FontFamily(
        Font(R.font.jetbrains_mono, FontWeight.Normal),
        Font(R.font.jetbrains_mono_medium, FontWeight.Medium),
    )

    /** Material Symbols Outlined, subset to the icons the app draws. */
    val Symbols = FontFamily(Font(R.font.material_symbols))

    /** The same icons filled, for the few the prototype fills: play, pause, stop. */
    val SymbolsFilled = FontFamily(Font(R.font.material_symbols_filled))

    /** Inter at the prototype's sizes. Line height follows the browser's default. */
    fun text(size: TextUnit, weight: FontWeight = FontWeight.Normal, color: Color = Forge.Text) =
        TextStyle(fontFamily = Inter, fontSize = size, fontWeight = weight, color = color)

    val material = Typography(
        bodyLarge = text(16.sp),
        bodyMedium = text(14.sp),
        bodySmall = text(12.sp),
        titleMedium = text(16.sp, FontWeight.SemiBold),
        labelLarge = text(14.sp, FontWeight.SemiBold),
    )
}

/** Names the icons the app may draw, so a typo is a compile error, not a blank. */
@Suppress("EnumEntryName")
enum class Sym {
    account_tree, add, add_circle, ads_click, arrow_back, arrow_forward, arrow_outward, auto_awesome,
    blur_on, bolt, brush, camera_alt, center_focus_strong, check, check_circle, chevron_right, close,
    code, content_copy, conversion_path, cottage, crop_free, crop_square, delete, deployed_code,
    directions_run, directions_walk, door_front, download, edit, emoji_objects, expand_less,
    expand_more, extension, fast_rewind, flip, flutter_dash, folder, folder_open, folder_special,
    forest, front_hand, grass, grid_4x4, grid_on, grid_view, home, image, info, keyboard_arrow_up,
    landscape, landslide, layers, library_add, light, lightbulb, link, local_florist, lock, more_vert,
    open_in_full, open_with, pan_tool, park, pause, photo_camera, photo_library, play_arrow,
    play_circle, progress_activity, public, radio_button_unchecked, redo, remove, route,
    rotate_right, sailing, select_all, settings, skip_next, speed, sports_soccer, star, stay_current_landscape,
    stay_current_portrait, stop, sunny, swap_vert, swipe_up, target, title, touch_app, tune, undo,
    vibration, view_day, view_in_ar, view_quilt, visibility, visibility_off, volume_up, water_drop,
    web_asset,
}

/**
 * One Material Symbol, drawn as the prototype draws it: the icon font's
 * ligature for its name. [filled] only works for play, pause and stop.
 */
@Composable
fun Symbol(
    icon: Sym,
    size: TextUnit = 22.sp,
    color: Color = Color.Unspecified,
    modifier: Modifier = Modifier,
    filled: Boolean = false,
) {
    Text(
        text = icon.name,
        modifier = modifier.size(size.value.dp),
        color = color,
        style = TextStyle(
            fontFamily = if (filled) ForgeType.SymbolsFilled else ForgeType.Symbols,
            fontSize = size,
            lineHeight = size,
            textAlign = TextAlign.Center,
        ),
        maxLines = 1,
        softWrap = false,
    )
}
