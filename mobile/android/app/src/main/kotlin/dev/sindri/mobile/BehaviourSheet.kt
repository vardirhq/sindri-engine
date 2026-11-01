package dev.sindri.mobile

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/**
 * Behaviour: rules made of When → Do → And blocks, which are Decay underneath.
 * The Code tab shows the script the blocks write, read-only, so the two never
 * disagree. The prototype's `sheet: 'behaviour'`.
 */
@Composable
fun BoxScope.BehaviourSheet(sheet: EditorSheet.Behaviour, onEvent: (EditorEvent) -> Unit) = SheetFrame(SheetHeight.Below(56.dp)) {
    val behaviour = sheet.behaviour
    SheetHeader("Behaviour", behaviour.subject, onBack = { onEvent(EditorEvent.OpenInspector) }) {
        Segmented(listOf("Blocks", "Code"), if (sheet.code) 1 else 0, { onEvent(if (it == 1) EditorEvent.BehaviourCode else EditorEvent.BehaviourBlocks) })
    }
    if (sheet.code) CodeView(behaviour, onEvent) else BlocksView(behaviour, onEvent)
}

@Composable
private fun androidx.compose.foundation.layout.ColumnScope.BlocksView(behaviour: BehaviourUi, onEvent: (EditorEvent) -> Unit) = SheetBody(horizontal = 12.dp) {
    behaviour.decay?.let { script ->
        DecayScriptView(script) { slot, value -> onEvent(EditorEvent.ScriptSlot(slot, value)) }
        return@SheetBody
    }
    if (behaviour.templates.isNotEmpty()) {
        Column {
            Text("Start from", style = ForgeType.text(12.sp, FontWeight.Medium, Forge.TextFaint), modifier = Modifier.padding(start = 4.dp, end = 4.dp, top = 2.dp, bottom = 8.dp))
            Row(Modifier.horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                behaviour.templates.forEach { template ->
                    val shape = RoundedCornerShape(14.dp)
                    Column(
                        Modifier
                            .width(170.dp)
                            .heightIn(min = 96.dp)
                            .clip(shape)
                            .background(if (template.chosen) Forge.MetalDeep else Forge.Card)
                            .border(1.dp, if (template.chosen) Forge.Metal else Forge.Line, shape)
                            .clickable {}
                            .padding(12.dp),
                        verticalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Symbol(template.icon, 22.sp, Forge.Metal)
                        Text(template.label, style = ForgeType.text(13.sp, FontWeight.SemiBold))
                        Text(template.description, style = ForgeType.text(11.sp, color = Forge.TextFaint).copy(lineHeight = 15.sp))
                    }
                }
            }
        }
    }
    behaviour.rules.forEach { rule -> RuleCard(rule) }
    Row(
        Modifier
            .fillMaxWidth()
            .height(52.dp)
            .clip(RoundedCornerShape(16.dp))
            .background(Forge.Raised)
            .border(1.dp, Forge.Line, RoundedCornerShape(16.dp))
            .clickable(role = Role.Button) {},
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
    ) {
        Symbol(Sym.add, 20.sp, Forge.Text)
        Text("New rule", style = ForgeType.text(14.sp, FontWeight.Medium))
    }
}

@Composable
private fun RuleCard(rule: RuleUi) {
    val shape = RoundedCornerShape(18.dp)
    Column(Modifier.fillMaxWidth().clip(shape).background(Forge.Card).border(1.dp, Forge.Line, shape)) {
        Row(
            Modifier.fillMaxWidth().background(Forge.Raised).padding(start = 12.dp, end = 4.dp, top = 10.dp, bottom = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Lead("WHEN", Forge.Metal)
            Row(
                Modifier
                    .weight(1f)
                    .heightIn(min = 44.dp)
                    .clip(RoundedCornerShape(12.dp))
                    .background(Forge.MetalDeep)
                    .border(1.dp, Forge.MetalLine, RoundedCornerShape(12.dp))
                    .clickable {}
                    .padding(horizontal = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                Symbol(rule.whenIcon, 20.sp, Forge.MetalBright)
                Text(rule.whenText, style = ForgeType.text(14.sp, FontWeight.Medium), modifier = Modifier.weight(1f))
                Symbol(Sym.swap_vert, 18.sp, Forge.MetalLine)
            }
            IconButton(Sym.delete, "Delete rule", {}, Modifier.width(40.dp), iconSize = 20.sp, color = Forge.TextFaint)
        }
        Hairline()
        Column(Modifier.padding(start = 12.dp, end = 4.dp, top = 12.dp, bottom = 12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            rule.actions.forEachIndexed { index, action ->
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Lead(if (index == 0) "DO" else "AND", Forge.TextFaint)
                    Row(
                        Modifier
                            .weight(1f)
                            .heightIn(min = 44.dp)
                            .clip(RoundedCornerShape(12.dp))
                            .background(Forge.Well)
                            .border(1.dp, Forge.Line, RoundedCornerShape(12.dp))
                            .padding(start = 10.dp, end = 6.dp, top = 4.dp, bottom = 4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Symbol(action.icon, 20.sp, Forge.TextMuted)
                        Text(action.text, style = ForgeType.text(14.sp), modifier = Modifier.weight(1f))
                        action.target?.let { target ->
                            Row(
                                Modifier
                                    .height(34.dp)
                                    .clip(RoundedCornerShape(10.dp))
                                    .background(Forge.MetalDeep)
                                    .border(1.dp, Forge.MetalLine, RoundedCornerShape(10.dp))
                                    .clickable {}
                                    .padding(horizontal = 10.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(4.dp),
                            ) {
                                Symbol(Sym.ads_click, 16.sp, Forge.MetalBright)
                                Text(target, style = ForgeType.text(13.sp, FontWeight.SemiBold, Forge.MetalBright))
                            }
                        }
                        action.value?.let { value ->
                            Text(
                                value,
                                style = ForgeType.text(13.sp, color = Forge.TextMuted),
                                modifier = Modifier.background(Forge.Raised, RoundedCornerShape(9.dp)).padding(horizontal = 10.dp, vertical = 7.dp),
                            )
                        }
                    }
                    IconButton(Sym.close, "Remove ${action.text}", {}, Modifier.width(36.dp), iconSize = 18.sp, color = Forge.TextFaint)
                }
            }
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Box(Modifier.width(38.dp))
                Row(
                    Modifier
                        .weight(1f)
                        .height(44.dp)
                        .clip(RoundedCornerShape(12.dp))
                        .dashed(Forge.LineStrong, 12f)
                        .clickable {}
                        .padding(horizontal = 12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Symbol(Sym.add, 20.sp, Forge.TextMuted)
                    Text("Then…", style = ForgeType.text(14.sp, color = Forge.TextMuted))
                }
                Box(Modifier.width(36.dp))
            }
        }
    }
}

@Composable
private fun Lead(text: String, color: Color) {
    Text(text, style = ForgeType.text(11.sp, FontWeight.Bold, color), modifier = Modifier.width(38.dp))
}

@Composable
private fun androidx.compose.foundation.layout.ColumnScope.CodeView(behaviour: BehaviourUi, onEvent: (EditorEvent) -> Unit) = SheetBody(horizontal = 12.dp, gap = 10.dp) {
    Row(Modifier.padding(horizontal = 4.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Symbol(Sym.check_circle, 18.sp, Forge.Success)
        Text(behaviour.codeFile, style = ForgeType.text(12.sp, color = Forge.TextMuted).copy(fontFamily = ForgeType.Mono), modifier = Modifier.weight(1f))
        Text("decay-lsp: no errors", style = ForgeType.text(12.sp, color = Forge.TextFaint))
    }
    Column(
        Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(14.dp))
            .background(Forge.Well)
            .border(1.dp, Forge.Hairline, RoundedCornerShape(14.dp))
            .horizontalScroll(rememberScrollState())
            .padding(vertical = 12.dp),
    ) {
        behaviour.code.forEachIndexed { index, line ->
            Row {
                Text(
                    "${index + 1}",
                    style = codeStyle(Forge.LineStrong),
                    textAlign = TextAlign.End,
                    modifier = Modifier.width(32.dp).padding(end = 12.dp),
                )
                Text(highlight(line), style = codeStyle(Forge.Text), softWrap = false, modifier = Modifier.padding(end = 14.dp))
            }
        }
    }
    Text(behaviour.codeNote, style = ForgeType.text(12.sp, color = Forge.TextFaint).copy(lineHeight = 18.sp), modifier = Modifier.padding(horizontal = 4.dp))
    Row(
        Modifier
            .fillMaxWidth()
            .height(48.dp)
            .clip(RoundedCornerShape(14.dp))
            .background(Forge.Raised)
            .border(1.dp, Forge.Line, RoundedCornerShape(14.dp))
            .clickable(role = Role.Button) {},
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.CenterHorizontally),
    ) {
        Symbol(Sym.code, 20.sp, Forge.Text)
        Text("Edit as code", style = ForgeType.text(14.sp))
    }
}

private fun codeStyle(color: Color) = ForgeType.text(11.5.sp, color = color).copy(fontFamily = ForgeType.Mono, lineHeight = 19.5.sp)

private val keywords = setOf("script", "fn", "let", "var", "if", "else", "return", "on", "emit", "event", "true", "false", "null", "while")
private val types = setOf("number", "bool", "String", "Entity")
private val token = Regex("""(//.*$)|("[^"]*")|(@export)|(\b\d+(?:\.\d+)?\b)|([A-Za-z_][A-Za-z0-9_]*)|(\s+)|(.)""")

/** Decay colored as the prototype colors it: keywords metal, types blue, strings green. */
private fun highlight(line: String): AnnotatedString = buildAnnotatedString {
    token.findAll(line).forEach { match ->
        val groups = match.groups
        val color = when {
            groups[1] != null -> Color(0xFF68717A)
            groups[2] != null -> Color(0xFF79B68A)
            groups[3] != null -> Color(0xFFE9A33A)
            groups[4] != null -> Color(0xFFE9C13A)
            groups[5] != null -> when {
                match.value in keywords -> Color(0xFFE9A33A)
                match.value in types -> Color(0xFF5B90D6)
                match.value.first().isUpperCase() -> Color(0xFFD9C29A)
                else -> Forge.Text
            }
            else -> Forge.Text
        }
        withStyle(SpanStyle(color = color)) { append(match.value) }
    }
}
