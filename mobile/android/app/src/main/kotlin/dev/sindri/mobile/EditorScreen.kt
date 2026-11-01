package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChanged
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * The editor: the scene fills the screen and everything else floats over it,
 * as the prototype's `screen: 'editor'` lays it out. [canvas] is the engine's
 * view of the scene in the app, and a picture of it in the design tests.
 */
@Composable
fun EditorScreen(ui: EditorUi, onEvent: (EditorEvent) -> Unit, canvas: @Composable BoxScope.() -> Unit) {
    Box(Modifier.fillMaxSize().background(Forge.Ground)) {
        canvas()
        if (ui.mode == PlayMode.Edit && ui.painting == null) ui.handles?.let { SelectionHandles(it, onEvent) }
        when (ui.mode) {
            PlayMode.Edit -> EditChrome(ui, onEvent)
            PlayMode.Play -> PlayChrome(ui.play, onEvent)
            PlayMode.Paused -> if (ui.sheet !is EditorSheet.Inspector) PausedPanel(ui.play, onEvent)
        }
        ui.sheet?.let { EditorSheetHost(it, onEvent) }
        ui.toast?.let { Toast(it, raised = ui.sheet == null, onEvent) }
    }
}

/** The glass the prototype floats over the canvas. */
fun Modifier.glass(shape: Shape): Modifier = clip(shape).background(Forge.Glass).border(1.dp, Forge.GlassLine, shape)

@Composable
private fun BoxScope.EditChrome(ui: EditorUi, onEvent: (EditorEvent) -> Unit) {
    // A full-height sheet covers the top bar, as in the prototype.
    val coversTop = ui.sheet is EditorSheet.Behaviour || (ui.sheet as? EditorSheet.Inspector)?.full == true
    Box(Modifier.fillMaxSize().statusBarsPadding()) {
        if (!coversTop) {
            TopLeft(ui, onEvent, Modifier.align(Alignment.TopStart).padding(10.dp))
            TopRight(ui, onEvent, Modifier.align(Alignment.TopEnd).padding(10.dp))
        }
        ui.menu?.let { OverflowMenu(it, onEvent, Modifier.align(Alignment.TopEnd).padding(top = 70.dp, end = 10.dp)) }
        ui.coach?.let { Coach(it, onEvent, Modifier.align(Alignment.TopCenter).padding(top = 72.dp, start = 10.dp, end = 10.dp)) }
    }
    Box(Modifier.fillMaxSize().navigationBarsPadding()) {
        val painting = ui.painting
        if (painting != null) {
            PaintBar(painting, onEvent)
        } else {
            ui.selection?.let { SelectionChip(it, onEvent, Modifier.align(Alignment.BottomCenter).padding(bottom = 98.dp)) }
            BottomBar(ui, onEvent)
        }
    }
}

@Composable
private fun TopLeft(ui: EditorUi, onEvent: (EditorEvent) -> Unit, modifier: Modifier) {
    Row(modifier.height(52.dp).glass(RoundedCornerShape(18.dp)), verticalAlignment = Alignment.CenterVertically) {
        IconButton(Sym.home, "Home", { onEvent(EditorEvent.Home) }, Modifier.width(46.dp), size = 52.dp)
        Box(Modifier.size(1.dp, 24.dp).background(Forge.Line))
        Row(
            Modifier
                .height(52.dp)
                .clickable(onClickLabel = "Scenes", onClick = { onEvent(EditorEvent.OpenScenes) })
                .padding(start = 12.dp, end = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Column {
                Text(ui.projectTitle, style = ForgeType.text(11.sp, color = Forge.TextFaint), maxLines = 1)
                Text(
                    if (ui.unsaved) "${ui.sceneName} •" else ui.sceneName,
                    style = ForgeType.text(14.sp, FontWeight.SemiBold),
                    maxLines = 1,
                )
            }
            Symbol(Sym.expand_more, 20.sp, Forge.TextFaint)
        }
    }
}

@Composable
private fun TopRight(ui: EditorUi, onEvent: (EditorEvent) -> Unit, modifier: Modifier) {
    Row(modifier.height(52.dp).glass(RoundedCornerShape(18.dp)), verticalAlignment = Alignment.CenterVertically) {
        IconButton(Sym.undo, "Undo", { onEvent(EditorEvent.Undo) }, size = 44.dp, color = Forge.Text, enabled = ui.canUndo)
        IconButton(Sym.redo, "Redo", { onEvent(EditorEvent.Redo) }, size = 44.dp, color = Forge.Text, enabled = ui.canRedo)
        IconButton(Sym.more_vert, "More", { onEvent(EditorEvent.ToggleMenu) }, size = 44.dp)
    }
}

@Composable
private fun OverflowMenu(menu: MenuUi, onEvent: (EditorEvent) -> Unit, modifier: Modifier) {
    Column(
        modifier
            .width(250.dp)
            .shadow(24.dp, RoundedCornerShape(16.dp))
            .clip(RoundedCornerShape(16.dp))
            .background(Forge.Popover)
            .border(1.dp, Forge.Line, RoundedCornerShape(16.dp))
            .padding(6.dp),
    ) {
        MenuRow(Sym.grid_4x4, "Snap to grid", menu.snap) { onEvent(EditorEvent.MenuToggle(MenuItem.Snap)) }
        MenuRow(Sym.lightbulb, "Lighting preview", menu.lightingPreview) { onEvent(EditorEvent.MenuToggle(MenuItem.LightingPreview)) }
        MenuRow(Sym.grid_on, "Show tile grid", menu.tileGrid) { onEvent(EditorEvent.MenuToggle(MenuItem.TileGrid)) }
        MenuRow(Sym.center_focus_strong, "Frame everything", null) { onEvent(EditorEvent.FrameAll) }
    }
}

@Composable
private fun MenuRow(icon: Sym, label: String, on: Boolean?, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().height(48.dp).clip(RoundedCornerShape(10.dp)).clickable(onClick = onClick).padding(horizontal = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Symbol(icon, 20.sp, Forge.TextMuted)
        Text(label, style = ForgeType.text(14.sp), modifier = Modifier.weight(1f))
        if (on != null) {
            Text(if (on) "On" else "Off", style = ForgeType.text(12.sp, FontWeight.SemiBold, if (on) Forge.MetalBright else Forge.TextFaint))
        }
    }
}

@Composable
private fun Coach(text: String, onEvent: (EditorEvent) -> Unit, modifier: Modifier) {
    Row(
        modifier
            .fillMaxWidth()
            .shadow(20.dp, RoundedCornerShape(14.dp))
            .clip(RoundedCornerShape(14.dp))
            .background(Forge.Popover)
            .border(1.dp, Forge.MetalLine, RoundedCornerShape(14.dp))
            .padding(start = 14.dp, top = 12.dp, bottom = 12.dp, end = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Symbol(Sym.lightbulb, 20.sp, Forge.Metal)
        Text(text, style = ForgeType.text(14.sp).copy(lineHeight = 20.sp), modifier = Modifier.weight(1f))
        IconButton(Sym.close, "Dismiss tip", { onEvent(EditorEvent.DismissCoach) }, Modifier.offset(y = (-8).dp), size = 36.dp, iconSize = 18.sp, color = Forge.TextFaint)
    }
}

@Composable
private fun SelectionChip(selection: SelectionUi, onEvent: (EditorEvent) -> Unit, modifier: Modifier) {
    val shape = RoundedCornerShape(21.dp)
    Row(
        modifier
            .height(42.dp)
            .widthIn(max = 388.dp)
            .clip(shape)
            .background(Forge.SelectionGlass)
            .border(1.dp, Forge.MetalLine, shape)
            .clickable(onClickLabel = "Open properties", onClick = { onEvent(EditorEvent.OpenInspector) })
            .padding(start = 14.dp, end = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Symbol(selection.icon, 18.sp, Forge.MetalBright)
        val name = if (selection.count > 1) "${selection.count} selected" else selection.name
        Text(name, style = ForgeType.text(14.sp, FontWeight.SemiBold), maxLines = 1, overflow = TextOverflow.Ellipsis)
        Text(selection.detail, style = ForgeType.text(12.sp, color = Forge.TextMuted), maxLines = 1)
        Symbol(Sym.keyboard_arrow_up, 20.sp, Forge.TextMuted)
    }
}

@Composable
private fun BoxScope.BottomBar(ui: EditorUi, onEvent: (EditorEvent) -> Unit) {
    val items = when {
        ui.selection == null -> listOf(BarItem.Add, BarItem.Paint, BarItem.Assets, BarItem.Layers, BarItem.Scenes)
        ui.selection.count > 1 -> listOf(BarItem.Properties, BarItem.Behaviour, BarItem.Duplicate, BarItem.Group, BarItem.Delete)
        else -> listOf(BarItem.Properties, BarItem.Behaviour, BarItem.Duplicate, BarItem.SelectMore, BarItem.Delete)
    }
    val open = when (ui.sheet) {
        is EditorSheet.Add -> BarItem.Add
        is EditorSheet.Assets -> BarItem.Assets
        is EditorSheet.Layers -> BarItem.Layers
        is EditorSheet.Scenes -> BarItem.Scenes
        else -> if (ui.selectingMore) BarItem.SelectMore else null
    }
    Row(
        Modifier
            .align(Alignment.BottomStart)
            .padding(start = 10.dp, end = 86.dp, bottom = 18.dp)
            .fillMaxWidth()
            .height(68.dp)
            .glass(RoundedCornerShape(22.dp))
            .padding(horizontal = 2.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceAround,
    ) { items.forEach { item -> BarButton(item, item == open) { onEvent(EditorEvent.Tool(item)) } } }
    PlayButton(Modifier.align(Alignment.BottomEnd).padding(end = 10.dp, bottom = 18.dp)) { onEvent(EditorEvent.Play) }
}

@Composable
private fun BarButton(item: BarItem, on: Boolean, onClick: () -> Unit) {
    val color = when {
        item.danger -> Forge.Danger
        on -> Forge.MetalBright
        else -> Forge.Text
    }
    Column(
        Modifier
            .size(54.dp, 58.dp)
            .clip(RoundedCornerShape(16.dp))
            .background(if (on) Forge.MetalDeep else Color.Transparent)
            .clickable(role = Role.Button, onClick = onClick),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(3.dp, Alignment.CenterVertically),
    ) {
        Symbol(item.icon, 24.sp, color)
        Text(item.label, style = ForgeType.text(10.5.sp, FontWeight.Medium, color), maxLines = 1, softWrap = false)
    }
}

@Composable
private fun PlayButton(modifier: Modifier, onClick: () -> Unit) {
    Box(
        modifier
            .size(68.dp)
            .shadow(16.dp, RoundedCornerShape(24.dp), ambientColor = Forge.Metal, spotColor = Forge.Metal)
            .clip(RoundedCornerShape(24.dp))
            .background(Forge.Metal)
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { contentDescription = "Play" },
        contentAlignment = Alignment.Center,
    ) { Symbol(Sym.play_arrow, 34.sp, Forge.OnMetal, filled = true) }
}

@Composable
private fun BoxScope.PaintBar(paint: PaintUi, onEvent: (EditorEvent) -> Unit) {
    Row(
        Modifier
            .align(Alignment.BottomCenter)
            .padding(bottom = 96.dp)
            .height(48.dp)
            .glass(RoundedCornerShape(16.dp))
            .padding(horizontal = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text("Size", style = ForgeType.text(12.sp, color = Forge.TextMuted), modifier = Modifier.padding(horizontal = 6.dp))
        listOf(1 to "S", 2 to "M", 3 to "L").forEach { (size, label) ->
            val on = paint.size == size
            Box(
                Modifier
                    .size(40.dp, 36.dp)
                    .clip(RoundedCornerShape(10.dp))
                    .background(if (on) Forge.MetalDeep else Color.Transparent)
                    .clickable(role = Role.RadioButton, onClick = { onEvent(EditorEvent.BrushSize(size)) }),
                contentAlignment = Alignment.Center,
            ) { Text(label, style = ForgeType.text(13.sp, FontWeight.SemiBold, if (on) Forge.MetalBright else Forge.TextMuted)) }
        }
        Box(Modifier.padding(horizontal = 4.dp).size(1.dp, 22.dp).background(Forge.Line))
        Row(
            Modifier
                .height(36.dp)
                .clip(RoundedCornerShape(10.dp))
                .background(if (paint.moveView) Forge.MetalDeep else Color.Transparent)
                .clickable(role = Role.Switch, onClick = { onEvent(EditorEvent.ToggleMoveView) })
                .padding(horizontal = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            val color = if (paint.moveView) Forge.MetalBright else Forge.TextMuted
            Symbol(Sym.pan_tool, 20.sp, color)
            Text("Move", style = ForgeType.text(13.sp, FontWeight.Medium, color))
        }
    }
    Row(
        Modifier
            .align(Alignment.BottomStart)
            .padding(start = 10.dp, end = 86.dp, bottom = 18.dp)
            .fillMaxWidth()
            .height(68.dp)
            .glass(RoundedCornerShape(22.dp))
            .padding(horizontal = 2.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceAround,
    ) { Brush.entries.forEach { brush -> BrushButton(brush, brush == paint.brush) { onEvent(EditorEvent.Brush(brush)) } } }
    Box(
        Modifier
            .align(Alignment.BottomEnd)
            .padding(end = 10.dp, bottom = 18.dp)
            .size(68.dp)
            .clip(RoundedCornerShape(24.dp))
            .background(Forge.MetalDeep)
            .border(1.dp, Forge.MetalLine, RoundedCornerShape(24.dp))
            .clickable(role = Role.Button, onClick = { onEvent(EditorEvent.DonePainting) })
            .semantics { contentDescription = "Done painting" },
        contentAlignment = Alignment.Center,
    ) { Symbol(Sym.check, 30.sp, Forge.MetalBright) }
}

@Composable
private fun BrushButton(brush: Brush, on: Boolean, onClick: () -> Unit) {
    Column(
        Modifier
            .size(60.dp, 58.dp)
            .clip(RoundedCornerShape(16.dp))
            .background(if (on) Forge.MetalDeep else Color.Transparent)
            .then(if (on) Modifier.border(1.dp, Forge.MetalLine, RoundedCornerShape(16.dp)) else Modifier)
            .clickable(role = Role.RadioButton, onClick = onClick),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(4.dp, Alignment.CenterVertically),
    ) {
        val swatch = Modifier.size(30.dp).clip(RoundedCornerShape(8.dp))
        when (brush) {
            Brush.Path -> Sprite(Sheet.GroundTile, cell = 3, row = 3, modifier = swatch)
            Brush.Grass -> Sprite(Sheet.GroundTile, cell = 0, modifier = swatch)
            Brush.Land -> Box(swatch) {
                Sprite(Sheet.GroundTile, cell = 0, modifier = Modifier.fillMaxSize())
                Box(Modifier.align(Alignment.BottomCenter).fillMaxWidth().height(6.dp).background(Color(0xFF2A2016)))
            }
            Brush.Erase -> Box(swatch.background(Color(0xFF06070A)).border(1.dp, Forge.LineStrong, RoundedCornerShape(8.dp)))
        }
        Text(brush.label, style = ForgeType.text(10.5.sp, FontWeight.Medium, if (on) Forge.MetalBright else Forge.Text))
    }
}

/**
 * The selection's ring and handles, as the prototype draws them over its
 * outline (which the engine draws round the selection's own shape): an
 * ellipse at its feet, a turn handle on a stem above it, and a scale handle
 * at its lower right. They keep their size on screen at any zoom.
 */
@Composable
private fun SelectionHandles(handles: HandlesUi, onEvent: (EditorEvent) -> Unit) {
    val turnY = handles.top - HANDLE_LIFT
    Box(
        Modifier
            .offset((handles.originX - 12).dp, (handles.originY - 5).dp)
            .size(24.dp, 10.dp)
            .border(1.5.dp, Forge.MetalBright, RoundedCornerShape(50)),
    )
    Box(
        Modifier
            .offset((handles.originX - 0.75f).dp, (turnY + HANDLE / 2).dp)
            .size(1.5.dp, (handles.top - turnY - HANDLE / 2).coerceAtLeast(0f).dp)
            .background(Forge.MetalBright),
    )
    Handle(HandleKind.Turn, handles.originX, turnY, Forge.Metal, Sym.rotate_right, "Turn", onEvent)
    Handle(HandleKind.Scale, handles.right + 6, handles.bottom, Forge.Text, Sym.open_in_full, "Scale", onEvent)
}

/** The prototype's handle: 34 across, round, with a drop shadow. */
private const val HANDLE = 34f

/** How far above the selection's top the turn handle sits. */
private const val HANDLE_LIFT = 30f

/** One round handle centred on [x], [y] in dp, which turns a drag into handle events in canvas pixels. */
@Composable
private fun Handle(kind: HandleKind, x: Float, y: Float, fill: Color, icon: Sym, name: String, onEvent: (EditorEvent) -> Unit) {
    val events by rememberUpdatedState(onEvent)
    var placed by remember { mutableStateOf<LayoutCoordinates?>(null) }
    Box(
        Modifier
            .offset((x - HANDLE / 2).dp, (y - HANDLE / 2).dp)
            .size(HANDLE.dp)
            .shadow(8.dp, CircleShape)
            .clip(CircleShape)
            .background(fill)
            .onGloballyPositioned { placed = it }
            .semantics { contentDescription = "$name handle" }
            .pointerInput(kind) {
                awaitEachGesture {
                    val down = awaitFirstDown()
                    down.consume()
                    val at = { position: Offset -> placed?.localToRoot(position) ?: position }
                    at(down.position).let { events(EditorEvent.HandleTaken(kind, it.x, it.y)) }
                    while (true) {
                        val change = awaitPointerEvent().changes.firstOrNull { it.id == down.id } ?: break
                        if (!change.pressed) break
                        if (change.positionChanged()) {
                            change.consume()
                            at(change.position).let { events(EditorEvent.HandleDragged(it.x, it.y)) }
                        }
                    }
                    events(EditorEvent.HandleReleased)
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        Symbol(icon, 18.sp, Forge.OnMetal)
    }
}

@Composable
private fun BoxScope.Toast(toast: ToastUi, raised: Boolean, onEvent: (EditorEvent) -> Unit) {
    Row(
        Modifier
            .align(if (raised) Alignment.BottomCenter else Alignment.TopCenter)
            .navigationBarsPadding()
            .padding(start = 12.dp, end = 12.dp, bottom = if (raised) 150.dp else 0.dp, top = if (raised) 0.dp else 12.dp)
            .shadow(20.dp, RoundedCornerShape(16.dp))
            .clip(RoundedCornerShape(16.dp))
            .background(Forge.Popover)
            .border(1.dp, Forge.Line, RoundedCornerShape(16.dp))
            .padding(start = 16.dp, end = 8.dp, top = 8.dp, bottom = 8.dp)
            .height(30.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text(toast.text, style = ForgeType.text(14.sp))
        if (toast.undo) {
            Box(
                Modifier
                    .height(34.dp)
                    .clip(RoundedCornerShape(10.dp))
                    .background(Forge.MetalDeep)
                    .clickable(onClick = { onEvent(EditorEvent.ToastUndo) })
                    .padding(horizontal = 12.dp),
                contentAlignment = Alignment.Center,
            ) { Text("Undo", style = ForgeType.text(13.sp, FontWeight.SemiBold, Forge.MetalBright)) }
        }
    }
}
