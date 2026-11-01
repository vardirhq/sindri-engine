package dev.sindri.mobile

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.LifecycleResumeEffect
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext

/** The project and scene the slice opens. */
private const val PROJECT = "lamplighter"
private const val SCENE = "world.scene"

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        setContent { ForgeTheme { SindriApp() } }
    }
}

/** Where the app is: the prototype's three screens. */
private sealed interface Screen {
    data object Dashboard : Screen
    data class NewGame(val step: NewGameStep) : Screen
    data object Editor : Screen
}

/** The native host, as the editor controller needs it. */
private object NativeHost : EditorController.Native {
    override fun command(command: String, vararg arguments: Any?): String? = NativeBridge.command(command, *arguments)
    override fun state(): EditorState = EditorState.read(NativeBridge.state())
}

@Composable
private fun SindriApp() {
    var screen by remember { mutableStateOf<Screen>(Screen.Dashboard) }
    val controller = remember { EditorController(NativeHost) }
    BackHandler(screen != Screen.Dashboard) {
        screen = when (val current = screen) {
            is Screen.NewGame -> if (current.step is NewGameStep.Name) Screen.NewGame(NewGameStep.Template) else Screen.Dashboard
            // In the editor, Back steps out of the menu, a sheet, painting,
            // a run and the selection before it leaves.
            Screen.Editor -> if (controller.back()) Screen.Editor else Screen.Dashboard
            else -> Screen.Dashboard
        }
    }
    when (val current = screen) {
        Screen.Dashboard -> DashboardScreen(
            game = GameCard("Lamplighter", "$SCENE · the bundled sample"),
            onContinue = { screen = Screen.Editor },
            onNewGame = { screen = Screen.NewGame(NewGameStep.Template) },
            onOpenFolder = {},
            onSample = {},
            onSettings = {},
        )
        is Screen.NewGame -> NewGameFlow(current.step, onStep = { screen = Screen.NewGame(it) }, onDone = { screen = Screen.Editor }, onExit = { screen = Screen.Dashboard })
        Screen.Editor -> EditorRoute(controller, onHome = { screen = Screen.Dashboard })
    }
}

@Composable
private fun NewGameFlow(step: NewGameStep, onStep: (NewGameStep) -> Unit, onDone: () -> Unit, onExit: () -> Unit) {
    if (step is NewGameStep.Creating) {
        // Creating a project is not wired yet: the steps play out, and the
        // bundled sample opens in its place.
        LaunchedEffect(step.done) {
            delay(380)
            if (step.done < CreateLines.size) onStep(step.copy(done = step.done + 1)) else onDone()
        }
    }
    NewGameScreen(
        step = step,
        onBack = { if (step is NewGameStep.Name) onStep(NewGameStep.Template) else onExit() },
        onTemplate = { if (it.available) onStep(NewGameStep.Name("Night Market", Orientation.Portrait)) },
        onName = { name -> (step as? NewGameStep.Name)?.let { onStep(it.copy(name = name)) } },
        onOrientation = { orientation -> (step as? NewGameStep.Name)?.let { onStep(it.copy(orientation = orientation)) } },
        onCreate = { (step as? NewGameStep.Name)?.let { onStep(NewGameStep.Creating(it.name, 0)) } },
    )
}

@Composable
private fun EditorRoute(controller: EditorController, onHome: () -> Unit) {
    val context = LocalContext.current
    val density = LocalDensity.current.density
    var surface by remember { mutableStateOf<SceneSurface?>(null) }
    var opened by remember { mutableStateOf(false) }

    LaunchedEffect(Unit) {
        val project = withContext(Dispatchers.IO) { runCatching { BundledProjects.install(context, PROJECT) } }
        project.onSuccess { directory ->
            // On the main thread, where the native host lives.
            NativeBridge.open(directory.path, SCENE)
            opened = true
            controller.opened(density)
        }
    }
    LifecycleResumeEffect(surface) {
        surface?.running = true
        onPauseOrDispose { surface?.running = false }
    }
    // While a game runs, its clock and whether it is still running are read
    // a few times a second; the frames themselves are the surface's.
    LaunchedEffect(controller.ui.mode) {
        while (controller.ui.mode != PlayMode.Edit) {
            delay(250)
            controller.refresh()
        }
    }
    LaunchedEffect(controller.ui.toast) {
        if (controller.ui.toast != null) {
            delay(2600)
            controller.dismissToast()
        }
    }
    EditorScreen(controller.ui, { controller.on(it, onHome) }) {
        AndroidView(
            modifier = Modifier.fillMaxSize(),
            factory = { viewContext ->
                SceneSurface(viewContext, onError = {}, onChanged = { settled -> if (opened) controller.canvasChanged(settled) }).also { created ->
                    created.running = true
                    surface = created
                }
            },
        )
    }
}
