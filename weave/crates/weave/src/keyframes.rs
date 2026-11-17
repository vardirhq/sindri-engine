//! `@keyframes`: an animation's values along its run, as CSS writes them.
//!
//! ```css
//! @keyframes pop {
//!     from { width: 120px; }
//!     40%  { width: 150px; }
//!     to   { width: 120px; }
//! }
//! .gathered { animation: pop 300ms ease-out; }
//! ```
//!
//! Weave keeps the stops as written; the host plays them, as it plays
//! transitions (`sindri-weave`).

use std::collections::BTreeMap;

use crate::ParseError;

/// One animation's stops: how far along its run each is, from 0 to 1, in
/// order, and the declarations it holds there.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Keyframes {
    pub stops: Vec<(f32, BTreeMap<String, String>)>,
}

/// Reads the body of `@keyframes name { … }`: blocks headed `from`, `to` or a
/// percentage, or a comma-separated list of them.
pub(crate) fn parse(body: &str) -> Result<Keyframes, ParseError> {
    let mut stops = Vec::new();
    let mut rest = body;
    while !rest.trim().is_empty() {
        let open = rest
            .find('{')
            .ok_or_else(|| ParseError::MissingBlock(rest.trim().to_owned()))?;
        let heads = rest[..open].trim();
        let (block, tail) = crate::take_block(&rest[open + 1..])?;
        let declarations = crate::parse_declarations(block)?;
        for head in heads.split(',').map(str::trim) {
            let at = offset(head).ok_or_else(|| ParseError::InvalidKeyframe(head.to_owned()))?;
            stops.push((at, declarations.clone()));
        }
        rest = tail;
    }
    stops.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok(Keyframes { stops })
}

fn offset(head: &str) -> Option<f32> {
    match head {
        "from" => Some(0.0),
        "to" => Some(1.0),
        _ => {
            let percent: f32 = head.strip_suffix('%')?.trim().parse().ok()?;
            (percent.is_finite() && (0.0..=100.0).contains(&percent)).then_some(percent / 100.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parse;

    #[test]
    fn keyframes_read_their_stops_in_order() {
        let sheet = parse(
            "@keyframes pop { to { width: 10px; } from, 50% { width: 2px; color: #fff; } }
             .a { animation: pop 1s; }",
        )
        .unwrap();
        let pop = &sheet.keyframes["pop"];
        let at: Vec<f32> = pop.stops.iter().map(|(at, _)| *at).collect();
        assert_eq!(at, [0.0, 0.5, 1.0]);
        assert_eq!(pop.stops[2].1["width"], "10px");
        assert_eq!(pop.stops[0].1["color"], "#fff");
        assert_eq!(sheet.rules.len(), 1, "and the rules after them still read");
    }

    #[test]
    fn a_stop_that_is_not_one_is_refused() {
        assert!(parse("@keyframes x { middle { width: 1px; } }").is_err());
        assert!(parse("@keyframes x { 140% { width: 1px; } }").is_err());
    }
}
