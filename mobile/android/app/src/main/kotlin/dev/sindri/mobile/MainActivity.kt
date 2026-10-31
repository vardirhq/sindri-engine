package dev.sindri.mobile

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.LifecycleResumeEffect
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** The project and scene the first slice opens. */
private const val PROJECT = "lamplighter"
private const val PROJECT_TITLE = "Lamplighter"
private const val SCENE = "world.scene"

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent { ForgeTheme { SceneScreen() } }
    }
}

/**
 * The canvas-first editor screen: the scene fills the screen, a calm top bar
 * with undo and redo sits over it, the selection is marked where it stands,
 * and its inspector rises from the bottom only while something is selected.
 */
@Composable
private fun SceneScreen() {
    val context = LocalContext.current
    var state by remember { mutableStateOf(EditorState(status = "Opening $PROJECT_TITLE…")) }
    var error by remember { mutableStateOf<String?>(null) }
    var surface by remember { mutableStateOf<SceneSurface?>(null) }
    val refresh = { state = EditorState.read(NativeBridge.state()) }
    val run = { result: String? ->
        error = result
        refresh()
    }

    LaunchedEffect(Unit) {
        val project = withContext(Dispatchers.IO) {
            runCatching { BundledProjects.install(context, PROJECT) }
        }
        project.fold(
            // On the main thread, where the native host lives.
            onSuccess = { directory -> run(NativeBridge.open(directory.path, SCENE)) },
            onFailure = { failure -> error = "Could not install $PROJECT_TITLE: ${failure.message}" },
        )
    }

    LifecycleResumeEffect(surface) {
        surface?.running = true
        refresh()
        onPauseOrDispose { surface?.running = false }
    }

    Box(Modifier.fillMaxSize().background(Forge.Ground)) {
        AndroidView(
            modifier = Modifier
                .fillMaxSize()
                .semantics { contentDescription = "Scene view of $SCENE" },
            factory = { viewContext ->
                SceneSurface(
                    viewContext,
                    onError = { message -> error = message },
                    onChanged = { refresh() },
                ).also { created ->
                    created.running = true
                    surface = created
                }
            },
        )
        state.marker?.let { (x, y) -> SelectionMarker(x, y) }
        Column(
            Modifier
                .fillMaxSize()
                .windowInsetsPadding(WindowInsets.safeDrawing)
                .imePadding()
                .padding(12.dp),
            verticalArrangement = Arrangement.SpaceBetween,
        ) {
            TopBar(
                undo = state.undo,
                redo = state.redo,
                unsaved = state.unsaved,
                onUndo = { run(NativeBridge.command("undo")) },
                onRedo = { run(NativeBridge.command("redo")) },
                onSave = { run(NativeBridge.command("save")) },
            )
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                error?.let { ErrorCard(it) }
                val inspector = state.inspector
                if (inspector != null) {
                    InspectorSheet(
                        inspector,
                        onSet = { type, field, value -> run(NativeBridge.command("setField", type, field, value)) },
                        onClose = { run(NativeBridge.command("deselect")) },
                    )
                } else {
                    StatusLine(state.status)
                }
            }
        }
    }
}

/** A ring where the selection's origin is, drawn over the canvas in surface pixels. */
@Composable
private fun SelectionMarker(x: Float, y: Float) {
    Canvas(Modifier.fillMaxSize()) {
        val center = Offset(x, y)
        drawCircle(Color.Black.copy(alpha = 0.5f), radius = 30f, center = center, style = Stroke(width = 9f))
        drawCircle(Forge.MetalBright, radius = 30f, center = center, style = Stroke(width = 5f))
        drawCircle(Forge.MetalBright, radius = 5f, center = center)
    }
}

@Composable
private fun TopBar(
    undo: String?,
    redo: String?,
    unsaved: Boolean,
    onUndo: () -> Unit,
    onRedo: () -> Unit,
    onSave: () -> Unit,
) {
    Row(
        Modifier
            .fillMaxWidth()
            .background(Forge.Panel, RoundedCornerShape(14.dp))
            .border(1.dp, Forge.Line, RoundedCornerShape(14.dp))
            .padding(horizontal = 14.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("◆", color = Forge.Metal, fontSize = 14.sp)
        Column(Modifier.weight(1f)) {
            Text(PROJECT_TITLE, color = Forge.Text, fontSize = 16.sp, fontWeight = FontWeight.SemiBold)
            Text(
                if (unsaved) "$SCENE · unsaved" else SCENE,
                color = if (unsaved) Forge.MetalBright else Forge.TextMuted,
                fontSize = 12.sp,
            )
        }
        HistoryButton("↶", undo?.let { "Undo $it" } ?: "Nothing to undo", undo != null, onUndo)
        HistoryButton("↷", redo?.let { "Redo $it" } ?: "Nothing to redo", redo != null, onRedo)
        HistoryButton("⤓", if (unsaved) "Save $SCENE" else "Saved", unsaved, onSave)
    }
}

@Composable
private fun HistoryButton(label: String, description: String, enabled: Boolean, onClick: () -> Unit) {
    Box(Modifier.semantics { contentDescription = description }) {
        if (enabled) {
            RoundButton(label, description, onClick)
        } else {
            Box(Modifier.padding(0.dp), contentAlignment = Alignment.Center) {
                Text(label, color = Forge.TextFaint, fontSize = 18.sp, modifier = Modifier.padding(12.dp))
            }
        }
    }
}

@Composable
private fun StatusLine(status: String) {
    Text(
        status,
        color = Forge.TextMuted,
        fontSize = 12.sp,
        modifier = Modifier
            .fillMaxWidth()
            .background(Forge.Panel, RoundedCornerShape(10.dp))
            .border(1.dp, Forge.Line, RoundedCornerShape(10.dp))
            .padding(horizontal = 12.dp, vertical = 8.dp),
    )
}

@Composable
private fun ErrorCard(message: String) {
    Text(
        message,
        color = Forge.Text,
        fontSize = 13.sp,
        modifier = Modifier
            .fillMaxWidth()
            .background(Color(0xF02A1414), RoundedCornerShape(10.dp))
            .border(1.dp, Forge.Danger, RoundedCornerShape(10.dp))
            .padding(12.dp),
    )
}
