package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.BoxWithConstraints
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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** While the game runs: stop, pause and the clock, and how many edits were made. */
@Composable
fun BoxScope.PlayChrome(play: PlayUi, onEvent: (EditorEvent) -> Unit) {
    play.stick?.let { Stick(it) }
    Box(Modifier.fillMaxSize().statusBarsPadding()) {
        Row(
            Modifier
                .align(Alignment.TopStart)
                .padding(10.dp)
                .height(52.dp)
                .clip(RoundedCornerShape(18.dp))
                .background(Color(0xBF0B0E12))
                .border(1.dp, Color(0x99252B32), RoundedCornerShape(18.dp))
                .padding(horizontal = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            IconButton(Sym.stop, "Stop", { onEvent(EditorEvent.Stop) }, Modifier.width(48.dp), iconSize = 24.sp, color = Forge.Text, filled = true)
            IconButton(Sym.pause, "Pause", { onEvent(EditorEvent.PauseOrResume) }, Modifier.width(48.dp), iconSize = 24.sp, color = Forge.Text, filled = true)
            Text(play.clock, style = ForgeType.text(13.sp, color = Forge.TextMuted), modifier = Modifier.padding(start = 4.dp, end = 10.dp))
        }
        if (play.runEdits.isNotEmpty()) {
            Row(
                Modifier
                    .align(Alignment.TopEnd)
                    .padding(top = 14.dp, end = 10.dp)
                    .height(44.dp)
                    .background(Forge.MetalDeep, RoundedCornerShape(16.dp))
                    .border(1.dp, Forge.MetalLine, RoundedCornerShape(16.dp))
                    .padding(horizontal = 14.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Symbol(Sym.edit, 18.sp, Forge.MetalBright)
                val count = play.runEdits.size
                Text("$count ${if (count == 1) "edit" else "edits"}", style = ForgeType.text(13.sp, FontWeight.SemiBold, Forge.MetalBright))
            }
        }
    }
    Text(
        "Drag anywhere to walk · pause to tune",
        style = ForgeType.text(13.sp, color = Forge.TextMuted),
        modifier = Modifier
            .align(Alignment.BottomCenter)
            .navigationBarsPadding()
            .padding(bottom = 40.dp)
            .background(Color(0xCC0B0E12), RoundedCornerShape(14.dp))
            .padding(horizontal = 16.dp, vertical = 10.dp),
    )
}

/** Paused: scrub back through what just ran, step, or resume; tap anything to tune it. */
@Composable
fun BoxScope.PausedPanel(play: PlayUi, onEvent: (EditorEvent) -> Unit) {
    Column(
        Modifier
            .align(Alignment.BottomCenter)
            .fillMaxWidth()
            .shadow(24.dp, RoundedCornerShape(topStart = 22.dp, topEnd = 22.dp))
            .clip(RoundedCornerShape(topStart = 22.dp, topEnd = 22.dp))
            .background(Forge.Sheet)
            .border(1.dp, Forge.Line, RoundedCornerShape(topStart = 22.dp, topEnd = 22.dp))
            .navigationBarsPadding()
            .padding(start = 16.dp, end = 16.dp, top = 16.dp, bottom = 22.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column {
            Text("Paused · ${play.clock}", style = ForgeType.text(15.sp, FontWeight.SemiBold))
            Text(
                "Drag back through the last ${play.recorded}. Tap anything to tune it.",
                style = ForgeType.text(12.sp, color = Forge.TextFaint),
                modifier = Modifier.padding(top = 2.dp),
            )
        }
        ScrubBar(play.scrub, play.runEdits.size)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            SquareButton(Sym.fast_rewind, "Step back") { onEvent(EditorEvent.StepBack) }
            SquareButton(Sym.skip_next, "Step one frame") { onEvent(EditorEvent.StepOnce) }
            Row(
                Modifier
                    .weight(1f)
                    .height(52.dp)
                    .clip(RoundedCornerShape(16.dp))
                    .background(Forge.Metal)
                    .clickable(role = Role.Button, onClick = { onEvent(EditorEvent.PauseOrResume) }),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp, Alignment.CenterHorizontally),
            ) {
                Symbol(Sym.play_arrow, 24.sp, Forge.OnMetal, filled = true)
                Text(if (play.scrub < 1f) "Resume from here" else "Resume", style = ForgeType.text(15.sp, FontWeight.SemiBold, Forge.OnMetal))
            }
        }
    }
}

@Composable
private fun ScrubBar(fraction: Float, marks: Int) {
    BoxWithConstraints(Modifier.fillMaxWidth().height(44.dp).semantics { contentDescription = "Recorded run" }) {
        val width = maxWidth
        Box(Modifier.offset(y = 20.dp).fillMaxWidth().height(4.dp).background(Forge.Line, RoundedCornerShape(2.dp)))
        Box(Modifier.offset(y = 20.dp).width(width * fraction).height(4.dp).background(Forge.Metal, RoundedCornerShape(2.dp)))
        repeat(marks) { index ->
            val at = (index + 1f) / (marks + 1f)
            Box(Modifier.offset(x = width * at - 1.dp, y = 14.dp).size(2.dp, 16.dp).background(Forge.MetalBright, RoundedCornerShape(1.dp)))
        }
        Box(
            Modifier
                .offset(x = width * fraction - 12.dp, y = 10.dp)
                .size(24.dp)
                .shadow(4.dp, RoundedCornerShape(12.dp))
                .background(Forge.Text, RoundedCornerShape(12.dp)),
        )
    }
}

@Composable
private fun SquareButton(icon: Sym, description: String, onClick: () -> Unit) {
    Box(
        Modifier
            .size(52.dp)
            .clip(RoundedCornerShape(16.dp))
            .background(Forge.Raised)
            .border(1.dp, Forge.Line, RoundedCornerShape(16.dp))
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { contentDescription = description },
        contentAlignment = Alignment.Center,
    ) { Symbol(icon, 22.sp, Forge.Text) }
}

/**
 * The design's stick, drawn where the thumb landed: a faint ring 112 across
 * and a 52 knob under the finger. The game reads it as held keys; this only
 * shows the player where they are pushing.
 */
@Composable
private fun Stick(stick: StickUi) {
    Box(
        Modifier
            .offset((stick.anchorX - 56).dp, (stick.anchorY - 56).dp)
            .size(112.dp)
            .clip(CircleShape)
            .background(Color(0x400B0E12))
            .border(2.dp, Color(0x40E8E4DC), CircleShape),
    )
    Box(
        Modifier
            .offset((stick.knobX - 26).dp, (stick.knobY - 26).dp)
            .size(52.dp)
            .clip(CircleShape)
            .background(Color(0xB3E8E4DC)),
    )
}
