package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.min
import androidx.compose.ui.unit.sp
import kotlin.math.min as minOf

/** Draws whichever sheet is open over the bottom of the canvas. */
@Composable
fun BoxScope.EditorSheetHost(sheet: EditorSheet, onEvent: (EditorEvent) -> Unit) {
    when (sheet) {
        is EditorSheet.Add -> AddSheet(sheet, onEvent)
        is EditorSheet.Assets -> AssetsSheet(sheet, onEvent)
        is EditorSheet.Import -> ImportSheet(sheet, onEvent)
        is EditorSheet.Layers -> LayersSheet(sheet, onEvent)
        is EditorSheet.Scenes -> ScenesSheet(sheet, onEvent)
        is EditorSheet.Inspector -> InspectorSheet(sheet, onEvent)
        is EditorSheet.Behaviour -> BehaviourSheet(sheet, onEvent)
        is EditorSheet.Review -> ReviewSheet(sheet, onEvent)
    }
}

/** How tall a sheet stands: a share of the screen, capped, as the prototype's `min(520px, 80%)`. */
sealed interface SheetHeight {
    data class Share(val max: Dp, val fraction: Float) : SheetHeight

    /** Up to [gap] below the top inset: the full inspector and Behaviour. */
    data class Below(val gap: Dp) : SheetHeight
    data object Wrap : SheetHeight
}

/** The sheet itself: rounded top, grab handle, then whatever it holds. */
@Composable
fun BoxScope.SheetFrame(height: SheetHeight, onHandle: (() -> Unit)? = null, content: @Composable ColumnScope.() -> Unit) {
    val shape = RoundedCornerShape(topStart = 22.dp, topEnd = 22.dp)
    BoxWithConstraints(Modifier.align(Alignment.BottomCenter).fillMaxSize().statusBarsPadding(), contentAlignment = Alignment.BottomCenter) {
        val sized = when (height) {
            is SheetHeight.Share -> Modifier.height(min(height.max, maxHeight * height.fraction))
            is SheetHeight.Below -> Modifier.height(maxHeight - height.gap)
            SheetHeight.Wrap -> Modifier
        }
        Column(
            sized
                .fillMaxWidth()
                .shadow(28.dp, shape)
                .clip(shape)
                .background(Forge.Sheet)
                .border(1.dp, Forge.Line, shape),
        ) {
            Box(
                Modifier
                    .fillMaxWidth()
                    .height(if (onHandle != null) 24.dp else 20.dp)
                    .then(if (onHandle != null) Modifier.clickable(onClickLabel = "Resize", onClick = onHandle) else Modifier),
                contentAlignment = Alignment.Center,
            ) { Box(Modifier.size(40.dp, 4.dp).background(Forge.LineStrong, RoundedCornerShape(2.dp))) }
            content()
        }
    }
}

/** A sheet's title row: back or nothing, title and subtitle, extra actions, close. */
@Composable
fun SheetHeader(
    title: String,
    subtitle: String? = null,
    onBack: (() -> Unit)? = null,
    onClose: (() -> Unit)? = null,
    mono: Boolean = false,
    actions: @Composable () -> Unit = {},
) {
    Row(
        Modifier.padding(start = if (onBack != null) 6.dp else 18.dp, end = 6.dp, bottom = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        if (onBack != null) IconButton(Sym.arrow_back, "Back", onBack, color = Forge.Text)
        Column(Modifier.weight(1f)) {
            Text(title, style = ForgeType.text(17.sp, FontWeight.SemiBold).let { if (mono) it.copy(fontFamily = ForgeType.Mono) else it })
            if (subtitle != null) Text(subtitle, style = ForgeType.text(12.sp, color = Forge.TextFaint), modifier = Modifier.padding(top = 2.dp))
        }
        actions()
        if (onClose != null) IconButton(Sym.close, "Close", onClose, color = Forge.TextFaint)
    }
}

/** The scrolling body of a sheet, with room for the gesture bar below it. */
@Composable
fun ColumnScope.SheetBody(horizontal: Dp = 14.dp, gap: Dp = 12.dp, content: @Composable ColumnScope.() -> Unit) {
    Column(
        Modifier
            .weight(1f)
            .verticalScroll(rememberScrollState())
            .navigationBarsPadding()
            .padding(start = horizontal, end = horizontal, bottom = 28.dp),
        verticalArrangement = Arrangement.spacedBy(gap),
        content = content,
    )
}

/**
 * A sprite fitted into a box, standing on its bottom edge and clipped to it, as
 * the prototype's thumbnails do. [zoom] enlarges sprites that are small in
 * their cell (a bush in a 112-pixel village cell), as the prototype's `tz`.
 */
@Composable
fun SpriteThumb(sheet: Sheet, cell: Int, box: Dp, modifier: Modifier = Modifier, zoom: Float = 1f) {
    val scale = minOf(box.value / sheet.cellWidth, box.value / sheet.cellHeight) * zoom
    Box(modifier.size(box).clip(RoundedCornerShape(0.dp)), contentAlignment = Alignment.BottomCenter) {
        // Larger than the box when zoomed: kept standing on the bottom edge
        // rather than centred, which is where Compose puts an oversized child.
        Sprite(
            sheet, cell,
            Modifier
                .wrapContentSize(Alignment.BottomCenter, unbounded = true)
                .size((sheet.cellWidth * scale).dp, (sheet.cellHeight * scale).dp),
        )
    }
}

@Composable
private fun <T> Grid(items: List<T>, columns: Int, gap: Dp = 8.dp, cell: @Composable (T) -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(gap)) {
        items.chunked(columns).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(gap)) {
                row.forEach { item -> Box(Modifier.weight(1f)) { cell(item) } }
                repeat(columns - row.size) { Box(Modifier.weight(1f)) }
            }
        }
    }
}

/** A card on a sheet: the prototype's `#12161B` with a hairline. */
fun Modifier.card(chosen: Boolean = false, radius: Dp = 14.dp): Modifier {
    val shape = RoundedCornerShape(radius)
    return clip(shape).background(if (chosen) Forge.MetalDeep else Forge.Card).border(1.dp, if (chosen) Forge.Metal else Forge.Hairline, shape)
}

@Composable
private fun BoxScope.AddSheet(sheet: EditorSheet.Add, onEvent: (EditorEvent) -> Unit) = SheetFrame(SheetHeight.Share(520.dp, 0.8f)) {
    SheetHeader("Add to world.scene", "Tap to place in view, or drag up into the world", onClose = { onEvent(EditorEvent.CloseSheet) })
    SheetBody {
        SectionLabel("This project's prefabs", Modifier.padding(start = 4.dp))
        Grid(sheet.prefabs, columns = 4) { prefab ->
            Column(
                Modifier.fillMaxWidth().card().clickable(onClickLabel = "Place ${prefab.name}") { onEvent(EditorEvent.PrefabChosen(prefab)) }.padding(horizontal = 4.dp, vertical = 8.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                SpriteThumb(prefab.sheet, prefab.cell, 60.dp, Modifier.padding(2.dp), zoom = prefab.zoom)
                Text(prefab.name, style = ForgeType.text(11.5.sp, color = Forge.TextMuted), maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        SectionLabel("Basics", Modifier.padding(start = 4.dp, top = 4.dp))
        val basics = listOf(
            Triple(Sym.sunny, "Light 2D", "A glow, no sprite") to true,
            Triple(Sym.deployed_code, "Empty", "A parent or a marker") to true,
            Triple(Sym.camera_alt, "Camera", "The scene already has a world camera") to false,
            Triple(Sym.title, "Text", "Goes on the Screen layer") to false,
        )
        Grid(basics, columns = 2) { (basic, available) ->
            Row(
                Modifier.fillMaxWidth().heightIn(min = 56.dp).alpha(if (available) 1f else 0.45f).card().clickable {}.padding(horizontal = 12.dp, vertical = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Symbol(basic.first, 20.sp, Forge.Text)
                Column {
                    Text(basic.second, style = ForgeType.text(13.sp, FontWeight.Medium))
                    Text(basic.third, style = ForgeType.text(11.sp, color = Forge.TextFaint).copy(lineHeight = 14.sp))
                }
            }
        }
    }
}

@Composable
private fun BoxScope.AssetsSheet(sheet: EditorSheet.Assets, onEvent: (EditorEvent) -> Unit) = SheetFrame(SheetHeight.Share(580.dp, 0.84f)) {
    SheetHeader("Assets", onClose = { onEvent(EditorEvent.CloseSheet) }) {
        Row(
            Modifier
                .height(40.dp)
                .clip(RoundedCornerShape(12.dp))
                .background(Forge.MetalDeep)
                .border(1.dp, Forge.MetalLine, RoundedCornerShape(12.dp))
                .clickable(role = Role.Button) { onEvent(EditorEvent.OpenImport) }
                .padding(horizontal = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Symbol(Sym.download, 18.sp, Forge.MetalBright)
            Text("Import", style = ForgeType.text(13.sp, FontWeight.SemiBold, Forge.MetalBright))
        }
    }
    Row(Modifier.horizontalScroll(rememberScrollState()).padding(start = 14.dp, end = 14.dp, bottom = 12.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        AssetTab.entries.forEach { tab -> Chip(tab.label, tab == sheet.tab, { onEvent(EditorEvent.AssetTabChosen(tab)) }) }
    }
    SheetBody(gap = 0.dp) {
        Grid(sheet.items, columns = 3) { item ->
            Column(
                Modifier.fillMaxWidth().card().clickable {}.padding(horizontal = 8.dp, vertical = 10.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(2.dp),
            ) {
                if (item.sheet != null) SpriteThumb(item.sheet, item.cell, 72.dp, zoom = item.zoom) else Symbol(item.icon, 32.sp, Forge.TextMuted)
                Text(item.name, style = ForgeType.text(12.sp, FontWeight.Medium), maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.padding(top = 4.dp))
                Text(item.path, style = ForgeType.text(11.sp, color = Forge.TextFaint), maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
    }
}

@Composable
private fun BoxScope.ImportSheet(sheet: EditorSheet.Import, onEvent: (EditorEvent) -> Unit) = SheetFrame(SheetHeight.Share(540.dp, 0.82f)) {
    SheetHeader("Import", onBack = { onEvent(EditorEvent.BackToAssets) })
    SheetBody(gap = 14.dp) {
        val sources = listOf(Sym.folder to "Files", Sym.photo_library to "Photos", Sym.photo_camera to "Camera", Sym.link to "Link")
        Grid(sources, columns = 4) { (icon, label) ->
            val first = label == "Files"
            Column(
                Modifier.fillMaxWidth().height(76.dp).card(first).clickable {},
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(6.dp, Alignment.CenterVertically),
            ) {
                val color = if (first) Forge.MetalBright else Forge.TextMuted
                Symbol(icon, 24.sp, color)
                Text(label, style = ForgeType.text(12.sp, color = color))
            }
        }
        SectionLabel("Downloads", Modifier.padding(start = 4.dp, top = 4.dp))
        Column {
            sheet.files.forEach { file ->
                Row(
                    Modifier.fillMaxWidth().heightIn(min = 68.dp).alpha(if (file.supported) 1f else 0.45f).clickable {}.padding(horizontal = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    Box(
                        Modifier.size(52.dp).clip(RoundedCornerShape(10.dp)).background(Forge.Well).border(1.dp, Forge.Hairline, RoundedCornerShape(10.dp)),
                        contentAlignment = Alignment.Center,
                    ) { if (file.sheet != null) Sprite(file.sheet, 0, Modifier.size(48.dp, 12.dp).width(48.dp)) }
                    Column(Modifier.weight(1f)) {
                        Text(file.name, style = ForgeType.text(14.sp).copy(fontFamily = ForgeType.Mono))
                        Text(file.detail, style = ForgeType.text(12.sp, color = Forge.TextFaint), modifier = Modifier.padding(top = 3.dp))
                    }
                }
                Hairline()
            }
        }
    }
}

@Composable
private fun BoxScope.LayersSheet(sheet: EditorSheet.Layers, onEvent: (EditorEvent) -> Unit) = SheetFrame(SheetHeight.Share(600.dp, 0.86f)) {
    SheetHeader("Layers", onClose = { onEvent(EditorEvent.CloseSheet) }) {
        Segmented(LayerTab.entries.map { it.label }, sheet.tab.ordinal, { onEvent(EditorEvent.LayerTabChosen(LayerTab.entries[it])) })
    }
    SheetBody(horizontal = 10.dp, gap = 0.dp) {
        when (sheet.tab) {
            LayerTab.Layers -> sheet.layers.forEach { layer ->
                Row(Modifier.fillMaxWidth().heightIn(min = 60.dp).padding(start = 8.dp, end = 4.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    Symbol(layer.icon, 22.sp, Forge.TextMuted)
                    Column(Modifier.weight(1f)) {
                        Text(layer.name, style = ForgeType.text(14.sp, FontWeight.Medium))
                        Text(layer.detail, style = ForgeType.text(12.sp, color = Forge.TextFaint), modifier = Modifier.padding(top = 2.dp))
                    }
                    val icon = when {
                        layer.locked -> Sym.lock
                        layer.visible -> Sym.visibility
                        else -> Sym.visibility_off
                    }
                    IconButton(icon, if (layer.visible) "Hide ${layer.name}" else "Show ${layer.name}", { onEvent(EditorEvent.LayerVisibility(layer.name)) }, size = 48.dp,
                        color = if (layer.locked || !layer.visible) Forge.LineStrong else Forge.TextMuted, enabled = !layer.locked)
                }
                Hairline()
            }
            LayerTab.Objects -> sheet.objects.forEach { row ->
                Row(
                    Modifier
                        .fillMaxWidth()
                        .heightIn(min = 52.dp)
                        .clip(RoundedCornerShape(12.dp))
                        .background(if (row.selected) Forge.MetalDeep else androidx.compose.ui.graphics.Color.Transparent)
                        .clickable { onEvent(EditorEvent.TreeRowTapped(row)) }
                        .padding(horizontal = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Box(Modifier.width((row.depth * 18).dp))
                    Box(Modifier.size(20.dp), contentAlignment = Alignment.Center) { row.chevron?.let { Symbol(it, 20.sp, Forge.TextFaint) } }
                    Symbol(row.icon, 20.sp, Forge.TextMuted)
                    Text(row.name, style = ForgeType.text(14.sp, color = if (row.selected) Forge.MetalBright else Forge.Text), modifier = Modifier.weight(1f), maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Text(row.detail, style = ForgeType.text(12.sp, color = Forge.TextFaint), modifier = Modifier.padding(end = 8.dp))
                }
            }
        }
    }
}

@Composable
private fun BoxScope.ScenesSheet(sheet: EditorSheet.Scenes, onEvent: (EditorEvent) -> Unit) = SheetFrame(SheetHeight.Share(520.dp, 0.8f)) {
    SheetHeader("Scenes", onClose = { onEvent(EditorEvent.CloseSheet) })
    SheetBody {
        Grid(sheet.scenes + null, columns = 2, gap = 10.dp) { scene ->
            val frame = if (scene == null) Modifier.clip(RoundedCornerShape(14.dp)).dashed(Forge.LineStrong, 14f) else Modifier.card(scene.current)
            Column(Modifier.fillMaxWidth().then(frame).clickable { if (scene != null) onEvent(EditorEvent.SceneTapped(scene)) else onEvent(EditorEvent.NewScene) }) {
                Box(Modifier.fillMaxWidth().height(96.dp).background(if (scene == null) androidx.compose.ui.graphics.Color.Transparent else Forge.Well), contentAlignment = Alignment.Center) {
                    when {
                        scene == null -> Text("+", style = ForgeType.text(28.sp, color = Forge.TextFaint))
                        scene.preview -> TiledSheet(Sheet.Ground, 64f, Modifier.fillMaxSize())
                        else -> Text("Lantern Crossing", style = ForgeType.text(13.sp, FontWeight.SemiBold, Forge.Text))
                    }
                }
                Row(Modifier.padding(start = 12.dp, end = 12.dp, top = 10.dp, bottom = 12.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    if (scene?.main == true) Symbol(Sym.home, 16.sp, Forge.Metal)
                    Text(scene?.name ?: "New scene", style = ForgeType.text(13.sp, FontWeight.Medium).copy(fontFamily = if (scene != null) ForgeType.Mono else ForgeType.Inter))
                }
            }
        }
        SectionLabel("Doors between scenes", Modifier.padding(start = 4.dp, top = 6.dp))
        sheet.doors.forEach { (from, to) ->
            Row(Modifier.fillMaxWidth().card().padding(horizontal = 14.dp, vertical = 12.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Text(from, style = ForgeType.text(13.sp).copy(fontFamily = ForgeType.Mono))
                Symbol(Sym.arrow_forward, 18.sp, Forge.Metal)
                Text(to, style = ForgeType.text(13.sp).copy(fontFamily = ForgeType.Mono))
                Text("Scene.go in shell.decay", style = ForgeType.text(13.sp, color = Forge.TextFaint), modifier = Modifier.weight(1f), textAlign = androidx.compose.ui.text.style.TextAlign.End)
            }
        }
    }
}

@Composable
private fun BoxScope.ReviewSheet(sheet: EditorSheet.Review, onEvent: (EditorEvent) -> Unit) = SheetFrame(SheetHeight.Wrap) {
    Column(Modifier.padding(start = 18.dp, end = 18.dp, bottom = 12.dp)) {
        Text("Keep what you changed?", style = ForgeType.text(17.sp, FontWeight.SemiBold))
        Text(
            "The scene is back as it was when you pressed Play. These are the edits you made while it ran.",
            style = ForgeType.text(13.sp, color = Forge.TextMuted).copy(lineHeight = 19.sp),
            modifier = Modifier.padding(top = 4.dp),
        )
    }
    Column(Modifier.padding(horizontal = 14.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        sheet.edits.forEachIndexed { index, edit ->
            Row(
                Modifier.fillMaxWidth().heightIn(min = 64.dp).card().clickable { onEvent(EditorEvent.KeepRunEdit(index)) }.padding(horizontal = 14.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Column(Modifier.weight(1f)) {
                    Text("${edit.name} · ${edit.label}", style = ForgeType.text(14.sp, FontWeight.Medium))
                    Row(Modifier.padding(top = 3.dp)) {
                        Text("${edit.from} → ", style = ForgeType.text(13.sp, color = Forge.TextMuted))
                        Text(edit.to, style = ForgeType.text(13.sp, color = Forge.MetalBright))
                    }
                }
                Toggle(edit.keep, { onEvent(EditorEvent.KeepRunEdit(index)) }, "Keep ${edit.name} ${edit.label}")
            }
        }
    }
    Row(Modifier.navigationBarsPadding().padding(start = 14.dp, end = 14.dp, top = 16.dp, bottom = 20.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        SecondaryButton("Discard all", { onEvent(EditorEvent.DiscardEdits) }, height = 54.dp)
        val kept = sheet.edits.count { it.keep }
        PrimaryButton(if (kept > 0) "Keep $kept" else "Keep none", { onEvent(EditorEvent.KeepEdits) }, Modifier.weight(1f), height = 54.dp)
    }
}
