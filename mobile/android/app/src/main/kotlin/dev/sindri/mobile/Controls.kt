package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** A small grey heading above a group: "Your games", "This project's prefabs". */
@Composable
fun SectionLabel(text: String, modifier: Modifier = Modifier) {
    Text(text, style = ForgeType.text(13.sp, FontWeight.Medium, Forge.TextFaint), modifier = modifier)
}

/** The big metal button: New game, Create game, Make prefab, Keep. */
@Composable
fun PrimaryButton(
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    icon: Sym? = null,
    height: Dp = 58.dp,
) {
    Row(
        modifier
            .height(height)
            .clip(RoundedCornerShape(18.dp))
            .background(Forge.Metal)
            .clickable(role = Role.Button, onClick = onClick),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.Center,
    ) {
        if (icon != null) {
            Symbol(icon, 24.sp, Forge.OnMetal)
            Box(Modifier.size(8.dp))
        }
        Text(label, style = ForgeType.text(16.sp, FontWeight.SemiBold, Forge.OnMetal))
    }
}

/** The quiet bordered button beside a primary one: Discard all, Swap. */
@Composable
fun SecondaryButton(label: String, onClick: () -> Unit, modifier: Modifier = Modifier, height: Dp = 52.dp) {
    Box(
        modifier
            .height(height)
            .clip(RoundedCornerShape(16.dp))
            .background(Forge.Raised)
            .border(1.dp, Forge.Line, RoundedCornerShape(16.dp))
            .clickable(role = Role.Button, onClick = onClick)
            .padding(horizontal = 16.dp),
        contentAlignment = Alignment.Center,
    ) { Text(label, style = ForgeType.text(14.sp, FontWeight.SemiBold)) }
}

/** An icon with a touch target around it and a spoken name. */
@Composable
fun IconButton(
    icon: Sym,
    description: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    size: Dp = 44.dp,
    iconSize: TextUnit = 22.sp,
    color: Color = Forge.TextMuted,
    enabled: Boolean = true,
    filled: Boolean = false,
) {
    Box(
        modifier
            .size(size)
            .clickable(enabled = enabled, role = Role.Button, onClick = onClick)
            .semantics { contentDescription = description },
        contentAlignment = Alignment.Center,
    ) { Symbol(icon, iconSize, if (enabled) color else Forge.LineStrong, filled = filled) }
}

/** A pill chip in a row of tabs: Prefabs, Sprites… Chosen ones glow metal. */
@Composable
fun Chip(label: String, chosen: Boolean, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val shape = RoundedCornerShape(18.dp)
    Box(
        modifier
            .height(36.dp)
            .clip(shape)
            .background(if (chosen) Forge.MetalDeep else Color.Transparent)
            .border(1.dp, if (chosen) Forge.MetalLine else Forge.Line, shape)
            .clickable(role = Role.Tab, onClick = onClick)
            .padding(horizontal = 14.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            label,
            style = ForgeType.text(13.sp, FontWeight.SemiBold, if (chosen) Forge.MetalBright else Forge.TextMuted),
        )
    }
}

/** Two or three choices in a well: Layers | Objects, Blocks | Code, Circle | Box. */
@Composable
fun Segmented(
    options: List<String>,
    chosen: Int,
    onChoose: (Int) -> Unit,
    modifier: Modifier = Modifier,
    fill: Boolean = false,
    icons: List<Sym>? = null,
    height: Dp = 34.dp,
) {
    Row(
        modifier
            .background(Forge.Well, RoundedCornerShape(12.dp))
            .border(1.dp, Forge.Hairline, RoundedCornerShape(12.dp))
            .padding(3.dp),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        options.forEachIndexed { index, label ->
            SegmentButton(label, index == chosen, { onChoose(index) }, if (fill) Modifier.weight(1f) else Modifier, icons?.getOrNull(index), height)
        }
    }
}

@Composable
private fun RowScope.SegmentButton(label: String, on: Boolean, onClick: () -> Unit, modifier: Modifier, icon: Sym?, height: Dp) {
    Row(
        modifier
            .height(height)
            .clip(RoundedCornerShape(9.dp))
            .background(if (on) Forge.MetalDeep else Color.Transparent)
            .clickable(role = Role.Tab, onClick = onClick)
            .padding(horizontal = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
    ) {
        val color = if (on) Forge.MetalBright else Forge.TextMuted
        if (icon != null) Symbol(icon, 20.sp, color)
        Text(label, style = ForgeType.text(13.sp, FontWeight.SemiBold, color))
    }
}

/** The prototype's switch: a 46×28 track with a knob that slides to metal. */
@Composable
fun Toggle(on: Boolean, onChange: (Boolean) -> Unit, description: String, modifier: Modifier = Modifier) {
    Box(
        modifier
            .size(46.dp, 28.dp)
            .clip(RoundedCornerShape(14.dp))
            .background(if (on) Forge.Metal else Forge.Line)
            .clickable(role = Role.Switch, onClick = { onChange(!on) })
            .semantics { contentDescription = description },
    ) {
        Box(
            Modifier
                .offset(x = if (on) 21.dp else 3.dp, y = 3.dp)
                .size(22.dp)
                .background(if (on) Forge.OnMetal else Forge.TextMuted, RoundedCornerShape(11.dp)),
        )
    }
}

/** A thin rule across a sheet. */
@Composable
fun Hairline(modifier: Modifier = Modifier) {
    Box(modifier.fillMaxWidth().height(1.dp).background(Forge.Hairline))
}
