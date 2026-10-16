//! Rich text: a string whose runs carry their own color, weight and slant.
//!
//! Written inline, because the words are the scene's and a span is part of
//! the words: `Hit for [color=#ffd34d][b]12[/b][/color]!`. Four tags and an
//! escape, and nothing else:
//!
//! - `[b]…[/b]` bold, `[i]…[/i]` italic.
//! - `[color=#rrggbb]…[/color]`, or `#rrggbbaa` with its own alpha.
//! - `[[` for a literal `[`.
//!
//! Anything else in brackets is drawn as written, for the reason a template's
//! stray brace is: text is content, and a typo should be on the screen where
//! it is seen, not silently eaten. A closing tag with nothing open is the same.
//! Tags nest; a `[/color]` returns to the color outside it.

/// One run of the string and how it is drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct TextSpan {
    pub text: String,
    /// The run's own color, or `None` for the string's.
    pub color: Option<[f32; 4]>,
    pub bold: bool,
    pub italic: bool,
}

/// Splits marked-up text into runs. Adjacent runs that would draw alike are
/// one run, and empty runs are dropped.
#[must_use]
pub fn rich_spans(markup: &str) -> Vec<TextSpan> {
    let mut spans: Vec<TextSpan> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let (mut bold, mut italic) = (0_u32, 0_u32);
    let mut text = String::new();
    let mut rest = markup;
    let flush =
        |text: &mut String, spans: &mut Vec<TextSpan>, style: (Option<[f32; 4]>, bool, bool)| {
            if text.is_empty() {
                return;
            }
            let (color, bold, italic) = style;
            if let Some(last) = spans.last_mut()
                && last.color == color
                && last.bold == bold
                && last.italic == italic
            {
                last.text.push_str(text);
            } else {
                spans.push(TextSpan {
                    text: text.clone(),
                    color,
                    bold,
                    italic,
                });
            }
            text.clear();
        };
    while let Some(open) = rest.find('[') {
        text.push_str(&rest[..open]);
        rest = &rest[open..];
        if let Some(after) = rest.strip_prefix("[[") {
            text.push('[');
            rest = after;
            continue;
        }
        let Some(close) = rest.find(']') else {
            break;
        };
        let tag = &rest[1..close];
        let style = (colors.last().copied(), bold > 0, italic > 0);
        let known = match tag {
            "b" => Some(Tag::Bold(true)),
            "/b" if bold > 0 => Some(Tag::Bold(false)),
            "i" => Some(Tag::Italic(true)),
            "/i" if italic > 0 => Some(Tag::Italic(false)),
            "/color" if !colors.is_empty() => Some(Tag::PopColor),
            _ => tag
                .strip_prefix("color=")
                .and_then(hex_color)
                .map(Tag::PushColor),
        };
        let Some(known) = known else {
            text.push_str(&rest[..=close]);
            rest = &rest[close + 1..];
            continue;
        };
        flush(&mut text, &mut spans, style);
        match known {
            Tag::Bold(true) => bold += 1,
            Tag::Bold(false) => bold -= 1,
            Tag::Italic(true) => italic += 1,
            Tag::Italic(false) => italic -= 1,
            Tag::PushColor(color) => colors.push(color),
            Tag::PopColor => {
                colors.pop();
            }
        }
        rest = &rest[close + 1..];
    }
    text.push_str(rest);
    flush(
        &mut text,
        &mut spans,
        (colors.last().copied(), bold > 0, italic > 0),
    );
    spans
}

enum Tag {
    Bold(bool),
    Italic(bool),
    PushColor([f32; 4]),
    PopColor,
}

/// `#rrggbb` or `#rrggbbaa`, as linear-agnostic 0..1 channels the way every
/// other color in a scene is written.
fn hex_color(text: &str) -> Option<[f32; 4]> {
    let digits = text.strip_prefix('#')?;
    if !matches!(digits.len(), 6 | 8) || !digits.is_ascii() {
        return None;
    }
    let channel = |index: usize| {
        digits
            .get(index * 2..index * 2 + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
            .map(|byte| f32::from(byte) / 255.0)
    };
    Some([
        channel(0)?,
        channel(1)?,
        channel(2)?,
        if digits.len() == 8 { channel(3)? } else { 1.0 },
    ])
}

#[cfg(test)]
mod tests {
    use super::{TextSpan, rich_spans};

    fn plain(text: &str) -> TextSpan {
        TextSpan {
            text: text.to_owned(),
            color: None,
            bold: false,
            italic: false,
        }
    }

    #[test]
    fn plain_text_is_one_run() {
        assert_eq!(rich_spans("Hello"), vec![plain("Hello")]);
        assert!(rich_spans("").is_empty());
    }

    #[test]
    fn tags_split_the_runs_and_nest() {
        let spans = rich_spans("a [b]b [i]c[/i][/b] [color=#ff0000]d[/color]");
        assert_eq!(spans.len(), 5);
        assert_eq!(spans[0], plain("a "));
        assert!(spans[1].bold && !spans[1].italic);
        assert!(spans[2].bold && spans[2].italic && spans[2].text == "c");
        assert_eq!(spans[3], plain(" "));
        assert_eq!(spans[4].color, Some([1.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn a_closing_color_returns_to_the_one_outside() {
        let spans = rich_spans("[color=#00ff00]a[color=#0000ff80]b[/color]c[/color]");
        assert_eq!(spans[0].color, Some([0.0, 1.0, 0.0, 1.0]));
        let inner = spans[1].color.expect("blue");
        assert!((inner[3] - 128.0 / 255.0).abs() < 1e-6);
        assert_eq!(spans[2].color, Some([0.0, 1.0, 0.0, 1.0]));
    }

    #[test]
    fn unknown_tags_and_stray_closers_are_drawn_as_written() {
        assert_eq!(
            rich_spans("[x]a[/b] [color=red]"),
            vec![plain("[x]a[/b] [color=red]")]
        );
        assert_eq!(rich_spans("[[b]] [b"), vec![plain("[b]] [b")]);
    }
}
