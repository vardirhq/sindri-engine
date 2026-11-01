package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** A kind of game New game can start from. */
data class GameTemplate(val name: String, val description: String, val tag: String?, val icon: Sym?, val available: Boolean)

val GameTemplates = listOf(
    GameTemplate("Top-down adventure", "Walk around a tile world. Starts with Wick, a camera and a grass tileset.", null, null, true),
    GameTemplate("Side-on platformer", "Run and jump with gravity and 2D physics.", "Not in prototype", Sym.directions_run, false),
    GameTemplate("Empty 2D", "A camera and nothing else.", "Not in prototype", Sym.crop_free, false),
    GameTemplate("Empty 3D", "Meshes, lights and GLB models. Touch editing for 3D comes later.", "Limited", Sym.view_in_ar, false),
)

enum class Orientation(val label: String, val icon: Sym) {
    Portrait("Portrait", Sym.stay_current_portrait),
    Landscape("Landscape", Sym.stay_current_landscape),
}

/** Where New game is: choosing a template, naming the game, or making it. */
sealed interface NewGameStep {
    data object Template : NewGameStep
    data class Name(val name: String, val orientation: Orientation) : NewGameStep

    /** [done] of [CreateLines] are finished; the next is in progress. */
    data class Creating(val name: String, val done: Int) : NewGameStep
}

val CreateLines = listOf(
    "Writing sindri.toml",
    "Making world.scene with a camera",
    "Adding Wick and the ground tileset",
    "Opening the scene",
)

/** New game: the prototype's `screen: 'new'`, steps 0, 1 and 2. */
@Composable
fun NewGameScreen(
    step: NewGameStep,
    onBack: () -> Unit,
    onTemplate: (GameTemplate) -> Unit,
    onName: (String) -> Unit,
    onOrientation: (Orientation) -> Unit,
    onCreate: () -> Unit,
) {
    Column(Modifier.fillMaxSize().background(Forge.Page).statusBarsPadding()) {
        Row(Modifier.padding(horizontal = 12.dp, vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
            IconButton(Sym.arrow_back, "Back", onBack, size = 48.dp, iconSize = 24.sp, color = Forge.Text)
            Text("New game", style = ForgeType.text(17.sp, FontWeight.SemiBold), modifier = Modifier.weight(1f).padding(start = 4.dp))
            val counter = when (step) {
                NewGameStep.Template -> "1 of 2"
                is NewGameStep.Name -> "2 of 2"
                is NewGameStep.Creating -> ""
            }
            Text(counter, style = ForgeType.text(13.sp, color = Forge.TextFaint), modifier = Modifier.padding(end = 12.dp))
        }
        when (step) {
            NewGameStep.Template -> TemplateStep(onTemplate)
            is NewGameStep.Name -> NameStep(step, onName, onOrientation, onCreate)
            is NewGameStep.Creating -> CreatingStep(step.done)
        }
    }
}

@Composable
private fun TemplateStep(onTemplate: (GameTemplate) -> Unit) {
    Column(
        Modifier.verticalScroll(rememberScrollState()).padding(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 32.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Heading("What are you making?", "Everything here can change later.", Modifier.padding(start = 6.dp, end = 6.dp, top = 4.dp, bottom = 6.dp))
        GameTemplates.forEach { template -> TemplateCard(template) { onTemplate(template) } }
    }
}

@Composable
private fun Heading(title: String, subtitle: String, modifier: Modifier = Modifier) {
    Column(modifier) {
        Text(title, style = ForgeType.text(24.sp, FontWeight.SemiBold))
        Text(subtitle, style = ForgeType.text(14.sp, color = Forge.TextMuted), modifier = Modifier.padding(top = 6.dp))
    }
}

@Composable
private fun TemplateCard(template: GameTemplate, onClick: () -> Unit) {
    val shape = RoundedCornerShape(18.dp)
    Row(
        Modifier
            .fillMaxWidth()
            .alpha(if (template.available) 1f else 0.7f)
            .clip(shape)
            .background(if (template.available) Forge.Card else Forge.Well)
            .border(1.dp, if (template.available) Forge.MetalLine else Forge.Hairline, shape)
            .clickable(onClickLabel = "Start a ${template.name}", onClick = onClick)
            .padding(12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Box(
            Modifier.size(76.dp).clip(RoundedCornerShape(12.dp)).background(Forge.Raised),
            contentAlignment = Alignment.Center,
        ) {
            if (template.icon != null) {
                Symbol(template.icon, 30.sp, Forge.TextMuted)
            } else {
                TiledSheet(Sheet.Ground, tile = 64f, modifier = Modifier.fillMaxSize())
                Sprite(Sheet.Wick, cell = 0, modifier = Modifier.offset(2.dp, 6.dp).size(72.dp, 66.dp).align(Alignment.TopStart))
            }
        }
        Column(Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(template.name, style = ForgeType.text(16.sp, FontWeight.SemiBold))
                if (template.tag != null) {
                    Text(
                        template.tag,
                        style = ForgeType.text(11.sp, FontWeight.SemiBold, Forge.TextFaint),
                        modifier = Modifier.background(Forge.Raised, RoundedCornerShape(6.dp)).padding(horizontal = 8.dp, vertical = 2.dp),
                    )
                }
            }
            Text(
                template.description,
                style = ForgeType.text(13.sp, color = Forge.TextMuted).copy(lineHeight = 18.sp),
                modifier = Modifier.padding(top = 4.dp),
            )
        }
    }
}

@Composable
private fun NameStep(step: NewGameStep.Name, onName: (String) -> Unit, onOrientation: (Orientation) -> Unit, onCreate: () -> Unit) {
    Column(
        Modifier.fillMaxSize().imePadding().navigationBarsPadding().padding(start = 22.dp, end = 22.dp, top = 8.dp, bottom = 24.dp),
        verticalArrangement = Arrangement.spacedBy(22.dp),
    ) {
        Heading("Name it", "It becomes the folder and the name in sindri.toml.")
        BasicTextField(
            value = step.name,
            onValueChange = onName,
            singleLine = true,
            textStyle = ForgeType.text(18.sp),
            cursorBrush = SolidColor(Forge.Metal),
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Words, imeAction = ImeAction.Done),
            modifier = Modifier.semantics { contentDescription = "Game name" },
            decorationBox = { field ->
                Box(
                    Modifier
                        .fillMaxWidth()
                        .height(58.dp)
                        .background(Forge.Well, RoundedCornerShape(14.dp))
                        .border(1.dp, Forge.MetalLine, RoundedCornerShape(14.dp))
                        .padding(horizontal = 16.dp),
                    contentAlignment = Alignment.CenterStart,
                ) { field() }
            },
        )
        Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
            SectionLabel("The game plays")
            Segmented(
                options = Orientation.entries.map { it.label },
                chosen = step.orientation.ordinal,
                onChoose = { onOrientation(Orientation.entries[it]) },
                fill = true,
                icons = Orientation.entries.map { it.icon },
                height = 48.dp,
                modifier = Modifier.fillMaxWidth(),
            )
        }
        Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
            SectionLabel("You start with")
            listOf(
                "world.scene, with a camera that follows Wick",
                "Wick, who walks and carries a lantern",
                "A grass and path tileset, ready to paint",
            ).forEach { line ->
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    Symbol(Sym.check, 20.sp, Forge.Success)
                    Text(line, style = ForgeType.text(14.sp, color = Forge.TextMuted))
                }
            }
        }
        Spacer(Modifier.weight(1f))
        PrimaryButton("Create game", onCreate, Modifier.fillMaxWidth())
    }
}

@Composable
private fun CreatingStep(done: Int) {
    Column(
        Modifier.fillMaxSize().padding(horizontal = 40.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp, Alignment.CenterVertically),
    ) {
        CreateLines.forEachIndexed { index, line ->
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                val (icon, color) = when {
                    index < done -> Sym.check_circle to Forge.Success
                    index == done -> Sym.progress_activity to Forge.Metal
                    else -> Sym.radio_button_unchecked to Forge.LineStrong
                }
                Symbol(icon, 20.sp, color)
                Text(line, style = ForgeType.text(14.sp, color = Forge.TextMuted))
            }
        }
    }
}
