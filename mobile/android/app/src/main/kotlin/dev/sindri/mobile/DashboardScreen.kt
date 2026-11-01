package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** A game the dashboard offers to learn from. */
data class SampleGame(val name: String, val summary: String, val icon: Sym)

val SampleGames = listOf(
    SampleGame("Lamplighter", "Top-down lantern adventure", Sym.emoji_objects),
    SampleGame("Platformer", "The 2D platformer", Sym.directions_run),
    SampleGame("Scorchball", "Couch football, 2-4 players", Sym.sports_soccer),
    SampleGame("Low Tide", "Top-down moving-home game", Sym.sailing),
    SampleGame("Orbital Last Stand", "Arena action, baked 3D sprites", Sym.public),
    SampleGame("Flappy", "One-button arcade", Sym.flutter_dash),
)

/**
 * The first screen: your games, samples to learn from, and New game. The
 * prototype's dashboard (`mobile/prototype/`, state `screen: 'dashboard'`).
 */
@Composable
fun DashboardScreen(
    game: GameCard,
    onContinue: () -> Unit,
    onNewGame: () -> Unit,
    onOpenFolder: () -> Unit,
    onSample: (SampleGame) -> Unit,
    onSettings: () -> Unit,
) {
    Box(Modifier.fillMaxSize().background(Forge.Page)) {
        Column(Modifier.fillMaxSize().statusBarsPadding()) {
            DashboardHeader(onSettings)
            Column(
                Modifier
                    .weight(1f)
                    .verticalScroll(rememberScrollState())
                    .padding(PaddingValues(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 130.dp)),
                verticalArrangement = Arrangement.spacedBy(22.dp),
            ) {
                Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    SectionLabel("Your games", Modifier.padding(start = 4.dp))
                    GameHero(game, onContinue)
                }
                Column {
                    SectionLabel("Samples to learn from", Modifier.padding(start = 4.dp, end = 4.dp, bottom = 6.dp))
                    SampleGames.forEach { sample -> SampleRow(sample) { onSample(sample) } }
                }
            }
        }
        DashboardActions(onNewGame, onOpenFolder, Modifier.align(Alignment.BottomCenter))
    }
}

/** What the dashboard knows about a game: its name and a line about it. */
data class GameCard(val name: String, val detail: String)

@Composable
private fun DashboardHeader(onSettings: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().padding(start = 22.dp, end = 8.dp, top = 10.dp, bottom = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(11.dp).rotate(45f).background(Forge.Metal, RoundedCornerShape(2.dp)))
        Spacer(Modifier.width(10.dp))
        Text("Sindri", style = ForgeType.text(21.sp, FontWeight.SemiBold), modifier = Modifier.weight(1f))
        IconButton(Sym.settings, "Settings", onSettings, size = 48.dp, iconSize = 24.sp)
    }
}

@Composable
private fun GameHero(game: GameCard, onContinue: () -> Unit) {
    val shape = RoundedCornerShape(18.dp)
    Column(
        Modifier
            .fillMaxWidth()
            .clip(shape)
            .background(Forge.Sheet)
            .border(1.dp, Forge.Hairline, shape)
            .clickable(onClickLabel = "Continue ${game.name}", onClick = onContinue),
    ) {
        Box(Modifier.fillMaxWidth().height(190.dp).clip(RoundedCornerShape(0.dp))) {
            TiledSheet(Sheet.Ground, tile = 128f, modifier = Modifier.fillMaxSize())
            // The prototype's scene: a house, a lit lamp post and Wick.
            Sprite(Sheet.Village, cell = 1, modifier = Modifier.offset((-10).dp, (-6).dp).size(168.dp))
            Sprite(Sheet.Objects, cell = 1, modifier = Modifier.offset(150.dp, 44.dp).size(128.dp))
            Sprite(Sheet.Wick, cell = 0, modifier = Modifier.offset(220.dp, 96.dp).size(96.dp, 88.dp))
        }
        Row(
            Modifier.padding(horizontal = 16.dp, vertical = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Column(Modifier.weight(1f)) {
                Text(game.name, style = ForgeType.text(17.sp, FontWeight.SemiBold))
                Text(
                    game.detail,
                    style = ForgeType.text(13.sp, color = Forge.TextMuted),
                    modifier = Modifier.padding(top = 3.dp),
                )
            }
            Row(
                Modifier
                    .height(44.dp)
                    .background(Forge.MetalDeep, RoundedCornerShape(12.dp))
                    .border(1.dp, Forge.MetalLine, RoundedCornerShape(12.dp))
                    .padding(horizontal = 16.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Symbol(Sym.play_arrow, 20.sp, Forge.MetalBright)
                Text("Continue", style = ForgeType.text(14.sp, FontWeight.SemiBold, Forge.MetalBright))
            }
        }
    }
}

@Composable
private fun SampleRow(sample: SampleGame, onClick: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .clickable(onClickLabel = "Open ${sample.name}", onClick = onClick)
            .padding(horizontal = 6.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Box(
            Modifier.size(44.dp).background(Forge.Raised, RoundedCornerShape(12.dp)),
            contentAlignment = Alignment.Center,
        ) { Symbol(sample.icon, 22.sp, Forge.TextMuted) }
        Column(Modifier.weight(1f)) {
            Text(sample.name, style = ForgeType.text(15.sp, FontWeight.Medium))
            Text(
                sample.summary,
                style = ForgeType.text(13.sp, color = Forge.TextFaint),
                modifier = Modifier.padding(top = 2.dp),
            )
        }
    }
    Box(Modifier.fillMaxWidth().height(1.dp).background(Forge.Hairline))
}

@Composable
private fun DashboardActions(onNewGame: () -> Unit, onOpenFolder: () -> Unit, modifier: Modifier) {
    Row(
        modifier
            .fillMaxWidth()
            .background(Brush.verticalGradient(0f to Forge.Page.copy(alpha = 0f), 0.4f to Forge.Page))
            .navigationBarsPadding()
            .padding(start = 16.dp, end = 16.dp, top = 22.dp, bottom = 20.dp),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        PrimaryButton("New game", onNewGame, Modifier.weight(1f), icon = Sym.add)
        Box(
            Modifier
                .size(58.dp)
                .clip(RoundedCornerShape(18.dp))
                .background(Forge.Raised)
                .border(1.dp, Forge.Line, RoundedCornerShape(18.dp))
                .clickable(onClickLabel = "Open a project folder", onClick = onOpenFolder),
            contentAlignment = Alignment.Center,
        ) { Symbol(Sym.folder_open, 24.sp, Forge.TextMuted) }
    }
}
