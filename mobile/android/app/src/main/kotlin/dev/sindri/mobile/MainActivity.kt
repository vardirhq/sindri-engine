package dev.sindri.mobile

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
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
import androidx.compose.ui.graphics.Color
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
 * The canvas-first editor screen, as far as the first slice goes: the scene
 * fills the screen, and a calm top bar and status line sit over it, clear of
 * the system bars.
 */
@Composable
private fun SceneScreen() {
    val context = LocalContext.current
    var status by remember { mutableStateOf("Opening $PROJECT_TITLE…") }
    var error by remember { mutableStateOf<String?>(null) }
    var surface by remember { mutableStateOf<SceneSurface?>(null) }

    LaunchedEffect(Unit) {
        val project = withContext(Dispatchers.IO) {
            runCatching { BundledProjects.install(context, PROJECT) }
        }
        project.fold(
            onSuccess = { directory ->
                // On the main thread, where the native host lives.
                error = NativeBridge.open(directory.path, SCENE)
                status = NativeBridge.status()
            },
            onFailure = { failure -> error = "Could not install $PROJECT_TITLE: ${failure.message}" },
        )
    }

    LifecycleResumeEffect(surface) {
        surface?.running = true
        status = NativeBridge.status()
        onPauseOrDispose { surface?.running = false }
    }

    Box(Modifier.fillMaxSize().background(Forge.Ground)) {
        AndroidView(
            modifier = Modifier
                .fillMaxSize()
                .semantics { contentDescription = "Scene view of $SCENE" },
            factory = { viewContext ->
                SceneSurface(viewContext) { message ->
                    error = message
                }.also { created ->
                    created.running = true
                    surface = created
                }
            },
        )
        Column(
            Modifier
                .fillMaxSize()
                .windowInsetsPadding(WindowInsets.safeDrawing)
                .padding(12.dp),
            verticalArrangement = Arrangement.SpaceBetween,
        ) {
            TopBar()
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                error?.let { ErrorCard(it) }
                StatusLine(status)
            }
        }
    }
}

@Composable
private fun TopBar() {
    Row(
        Modifier
            .fillMaxWidth()
            .background(Forge.Panel, RoundedCornerShape(14.dp))
            .border(1.dp, Forge.Line, RoundedCornerShape(14.dp))
            .padding(horizontal = 14.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Text("◆", color = Forge.Metal, fontSize = 14.sp)
        Column(Modifier.weight(1f)) {
            Text(PROJECT_TITLE, color = Forge.Text, fontSize = 16.sp, fontWeight = FontWeight.SemiBold)
            Text(SCENE, color = Forge.TextMuted, fontSize = 12.sp)
        }
        Chip("View only")
    }
}

@Composable
private fun Chip(label: String) {
    Text(
        label,
        color = Forge.MetalBright,
        fontSize = 12.sp,
        fontWeight = FontWeight.Medium,
        modifier = Modifier
            .background(Forge.MetalDeep, RoundedCornerShape(999.dp))
            .border(1.dp, Forge.Metal.copy(alpha = 0.4f), RoundedCornerShape(999.dp))
            .padding(horizontal = 10.dp, vertical = 4.dp),
    )
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
