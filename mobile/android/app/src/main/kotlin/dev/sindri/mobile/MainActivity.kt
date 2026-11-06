package dev.sindri.mobile

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.compose.LifecycleResumeEffect
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import kotlinx.coroutines.launch

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
    data class Editor(val project: String) : Screen
}

/** The native host, as the editor controller needs it. */
private object NativeHost : EditorController.Native {
    override fun command(command: String, vararg arguments: Any?): String? = NativeBridge.command(command, *arguments)
    override fun state(): EditorState = EditorState.read(NativeBridge.state())
    override fun script(): DecayScript? = DecayScript.read(NativeBridge.script())
}

@Composable
private fun SindriApp() {
    var screen by remember { mutableStateOf<Screen>(Screen.Dashboard) }
    val context = LocalContext.current
    val preferences = remember { context.getSharedPreferences("editor", 0) }
    val base = remember { (context.getExternalFilesDir(null) ?: context.filesDir).resolve("projects") }
    var projects by remember { mutableStateOf(LocalProjects.list(base)) }
    val currentId = (screen as? Screen.Editor)?.project ?: PROJECT
    val title = projects.firstOrNull { it.id == currentId }?.name ?: "Lamplighter"
    val sceneKey = if (currentId == PROJECT) "scene" else "scene:$currentId"
    val controller = remember(currentId) { EditorController(NativeHost, assetRoot = base.resolve("$currentId/assets").absolutePath + "/", onSceneOpened = { name -> preferences.edit().putString(sceneKey, name).commit() }, projectTitle = title, images = { ImportedImages.list(base.resolve(currentId)) }) }
    LaunchedEffect(screen) {
        if (screen == Screen.Dashboard) projects = withContext(Dispatchers.IO) { LocalProjects.list(base) }
    }
    BackHandler(screen != Screen.Dashboard) {
        screen = when (val current = screen) {
            is Screen.NewGame -> if (current.step is NewGameStep.Creating) current else if (current.step is NewGameStep.Name) Screen.NewGame(NewGameStep.Template) else Screen.Dashboard
            // In the editor, Back steps out of the menu, a sheet, painting,
            // a run and the selection before it leaves.
            is Screen.Editor -> if (controller.back()) current else Screen.Dashboard
            else -> Screen.Dashboard
        }
    }
    when (val current = screen) {
        Screen.Dashboard -> DashboardScreen(
            game = GameCard("Lamplighter", "$SCENE · the bundled sample"),
            onContinue = { screen = Screen.Editor(PROJECT) },
            onNewGame = { screen = Screen.NewGame(NewGameStep.Template) },
            onOpenFolder = {},
            onSample = {},
            onSettings = {},
            projects = projects.map { GameCard(it.name, "world.scene", it.id) },
            onProject = { game -> game.id?.let { screen = Screen.Editor(it) } },
        )
        is Screen.NewGame -> NewGameFlow(current.step, base, onStep = { screen = Screen.NewGame(it) }, onDone = { project -> projects = LocalProjects.list(base); screen = Screen.Editor(project.id) }, onExit = { screen = Screen.Dashboard })
        is Screen.Editor -> EditorRoute(controller, current.project, projects.firstOrNull { it.id == current.project }?.directory, onHome = { screen = Screen.Dashboard })
    }
}

@Composable
private fun NewGameFlow(step: NewGameStep, base: java.io.File, onStep: (NewGameStep) -> Unit, onDone: (LocalProjects.Project) -> Unit, onExit: () -> Unit) {
    var error by remember { mutableStateOf<String?>(null) }
    if (step is NewGameStep.Creating) {
        LaunchedEffect(step.name) {
            val result = withContext(Dispatchers.IO) { runCatching { LocalProjects.create(base, step.name) } }
            result.onSuccess(onDone).onFailure { error = it.message ?: "Could not create the game." }
        }
    }
    error?.let { message -> AlertDialog(onDismissRequest = {}, title = { Text("Could not create game") }, text = { Text(message) }, confirmButton = { TextButton(onClick = { error = null; onStep(NewGameStep.Name((step as? NewGameStep.Creating)?.name.orEmpty(), Orientation.Portrait)) }) { Text("Back") } }) }
    NewGameScreen(
        step = step,
        onBack = { if (step is NewGameStep.Name) onStep(NewGameStep.Template) else if (step !is NewGameStep.Creating) onExit() },
        onTemplate = { if (it.available) onStep(NewGameStep.Name("My game", Orientation.Portrait)) },
        onName = { name -> (step as? NewGameStep.Name)?.let { onStep(it.copy(name = name)) } },
        onOrientation = { orientation -> (step as? NewGameStep.Name)?.let { onStep(it.copy(orientation = orientation)) } },
        onCreate = { (step as? NewGameStep.Name)?.let { onStep(NewGameStep.Creating(it.name, 0)) } },
    )
}

@Composable
private fun EditorRoute(controller: EditorController, projectId: String, localProject: java.io.File?, onHome: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val directory = localProject ?: (context.getExternalFilesDir(null) ?: context.filesDir).resolve("projects/$PROJECT")
    val importImage = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) scope.launch {
            val imported = withContext(Dispatchers.IO) { runCatching { ImportedImages.import(directory, requireNotNull(context.contentResolver.openInputStream(uri)) { "Could not read the image." }) } }
            imported.onSuccess { file ->
                val failure = NativeBridge.command("reloadAssets")
                if (failure != null) file.delete()
                controller.importResult(failure ?: "Image imported. Tap it to use it.")
            }.onFailure { controller.importResult(it.message ?: "Could not import the image.") }
        }
    }
    val density = LocalDensity.current.density
    val lastScene = remember(projectId) { context.getSharedPreferences("editor", 0).getString(if (projectId == PROJECT) "scene" else "scene:$projectId", SCENE) ?: SCENE }
    var surface by remember { mutableStateOf<SceneSurface?>(null) }
    var opened by remember { mutableStateOf(false) }
    var openError by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(projectId) {
        val project = withContext(Dispatchers.IO) { runCatching { if (projectId == PROJECT) BundledProjects.install(context, PROJECT) else requireNotNull(localProject) { "Project no longer exists." } } }
        project.onSuccess { directory ->
            // On the main thread, where the native host lives.
            val failure = NativeBridge.open(directory.path, lastScene)
                ?: NativeBridge.command("recovery", context.filesDir.resolve(if (projectId == PROJECT) "scene-recovery" else "project-recovery/$projectId").path)
            if (failure == null) {
                opened = true
                controller.opened(density)
            } else {
                openError = failure
            }
        }.onFailure { openError = it.message ?: "Could not open the project." }
    }
    LifecycleResumeEffect(surface) {
        surface?.running = true
        onPauseOrDispose {
            surface?.running = false
            if (opened) controller.background()
        }
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
    if (controller.recoveryConflict) {
        AlertDialog(
            onDismissRequest = {},
            title = { Text("Scene changed") },
            text = { Text("The scene on disk changed since unsaved edits were recovered. Restore the recovered edits and replace the current scene when saved, or keep the current scene and discard the recovery?") },
            confirmButton = { TextButton(onClick = { controller.chooseRecovery(true) }) { Text("Restore edits") } },
            dismissButton = { TextButton(onClick = { controller.chooseRecovery(false) }) { Text("Keep current") } },
        )
    }
    openError?.let { message ->
        AlertDialog(
            onDismissRequest = {},
            title = { Text("Could not open scene") },
            text = { Text(message) },
            confirmButton = { TextButton(onClick = onHome) { Text("Back") } },
        )
    }
    EditorScreen(controller.ui, { event -> if (event == EditorEvent.OpenImport) importImage.launch(arrayOf("image/png")) else controller.on(event, onHome) }) {
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
