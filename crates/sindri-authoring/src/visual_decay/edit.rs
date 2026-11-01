//! Changing a script from its blocks: one node's text replaced, nothing else.

use decay_semantic::Environment;
use decay_syntax::{TokenKind, lex, parse};

use super::Span;
use super::structure::{Shape, reshape};

/// A change a block makes to the script, at the span of the slot it edits.
#[derive(Clone, Debug, PartialEq)]
pub enum ScriptEdit {
    /// A number slot's new value.
    Number { span: Span, value: f64 },
    /// A text slot's new value.
    Text { span: Span, value: String },
    /// A switch slot flipped.
    Bool { span: Span, value: bool },
}

/// `source` with `edit` made: the text of the one slot it names replaced,
/// and every other byte as it was.
///
/// # Errors
/// A span that is not in the script, one that does not hold the kind of
/// literal the edit is for (the script changed since its blocks were read),
/// a value that cannot be written (a number that is not finite), or a result
/// the parser cannot read — each in words for the app. A script that already
/// has syntax errors is not edited.
pub fn edit(source: &str, edit: &ScriptEdit) -> Result<String, String> {
    if !parse(source).diagnostics.is_empty() {
        return Err("the script has errors to fix as text first".to_owned());
    }
    let span = match edit {
        ScriptEdit::Number { span, .. }
        | ScriptEdit::Text { span, .. }
        | ScriptEdit::Bool { span, .. } => *span,
    };
    let old = source
        .get(span[0]..span[1])
        .ok_or("that part of the script is no longer there")?;
    let replacement = match edit {
        ScriptEdit::Number { value, .. } => {
            // A negative number is a negation of a literal, which the slot
            // shows as one number and replaces whole.
            let digits = old.trim_start_matches(['-', ' ', '\t']);
            if !matches!(only_token(digits), Some(TokenKind::Number(_))) {
                return Err("that is no longer a number in the script".to_owned());
            }
            number(*value, old.contains('.'))?
        }
        ScriptEdit::Text { value, .. } => {
            if !matches!(only_token(old), Some(TokenKind::String(_))) {
                return Err("that is no longer text in the script".to_owned());
            }
            text(value)
        }
        ScriptEdit::Bool { value, .. } => {
            if !matches!(only_token(old), Some(TokenKind::True | TokenKind::False)) {
                return Err("that is no longer a switch in the script".to_owned());
            }
            value.to_string()
        }
    };
    let mut edited = String::with_capacity(source.len() + replacement.len());
    edited.push_str(&source[..span[0]]);
    edited.push_str(&replacement);
    edited.push_str(&source[span[1]..]);
    let reparsed = parse(&edited);
    if let Some(problem) = reparsed.diagnostics.first() {
        return Err(format!("that would not read: {}", problem.message));
    }
    Ok(edited)
}

/// `source` with its shape changed from a node menu ([`Shape`]): whole
/// statements deleted, moved or added, and nothing else touched.
///
/// Checked twice: the result must parse, and it must not have more problems
/// than the script had before against `environment` (the project's scripts
/// and the engine's API, as `ScriptSources::environment` gives it), so
/// deleting a `let` something still reads, or adding a call to a function
/// that takes arguments, is refused with the reason rather than found at Play.
///
/// # Errors
/// A script with syntax errors (fixed as text first), a statement that is no
/// longer where its node said, a move past the end of its block, or a result
/// that does not read or does not check — each in words for the app.
pub fn change(source: &str, shape: &Shape, environment: &Environment) -> Result<String, String> {
    if !parse(source).diagnostics.is_empty() {
        return Err("the script has errors to fix as text first".to_owned());
    }
    let changed = reshape(source, shape)?;
    if let Some(problem) = parse(&changed).diagnostics.first() {
        return Err(format!("that would not read: {}", problem.message));
    }
    let before = decay_semantic::analyze_with_environment(source, environment).diagnostics;
    let after = decay_semantic::analyze_with_environment(&changed, environment).diagnostics;
    if after.len() > before.len() {
        let new = after
            .iter()
            .find(|problem| !before.iter().any(|old| old.message == problem.message))
            .unwrap_or(&after[0]);
        return Err(format!("that would not work: {}", new.message));
    }
    Ok(changed)
}

/// The one token `text` is, or `None` for none or several.
fn only_token(text: &str) -> Option<TokenKind> {
    let lexed = lex(text);
    if !lexed.diagnostics.is_empty() {
        return None;
    }
    let mut kinds = lexed
        .tokens
        .into_iter()
        .map(|token| token.kind)
        .filter(|kind| !matches!(kind, TokenKind::Eof));
    let first = kinds.next()?;
    kinds.next().is_none().then_some(first)
}

/// `value` as Decay writes a number: digits, a `.` and more digits where
/// there is a fraction (or where the old literal had one, to keep its style),
/// and a leading `-` for a negative, which Decay reads as negation.
fn number(value: f64, decimal: bool) -> Result<String, String> {
    if !value.is_finite() {
        return Err("a number has to be finite".to_owned());
    }
    let magnitude = value.abs();
    let mut written = if magnitude.fract() == 0.0 && magnitude < 1e15 {
        if decimal {
            format!("{magnitude:.1}")
        } else {
            format!("{magnitude:.0}")
        }
    } else {
        // Shortest round trip, never in exponent form, which Decay lacks.
        let shortest = format!("{magnitude}");
        if shortest.contains('e') {
            format!("{magnitude:.6}").trim_end_matches('0').to_owned()
        } else {
            shortest
        }
    };
    if value < 0.0 {
        written.insert(0, '-');
    }
    Ok(written)
}

/// `value` as a Decay string literal, with the escapes the language has.
fn text(value: &str) -> String {
    let mut written = String::with_capacity(value.len() + 2);
    written.push('"');
    for character in value.chars() {
        match character {
            '"' => written.push_str("\\\""),
            '\\' => written.push_str("\\\\"),
            '\n' => written.push_str("\\n"),
            '\r' => written.push_str("\\r"),
            '\t' => written.push_str("\\t"),
            other => written.push(other),
        }
    }
    written.push('"');
    written
}
