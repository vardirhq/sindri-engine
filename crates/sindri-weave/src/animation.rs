//! CSS animations: an element plays an `@keyframes` animation that its
//! `animation` declaration names, from the moment that declaration appears.
//!
//! ```css
//! @keyframes pop { 40% { width: 150px; } }
//! .gathered { animation: pop 300ms ease-out; }
//! ```
//!
//! `animation` takes a name, a duration, then in any order an easing, a delay,
//! an iteration count (`infinite` for ever) and a direction (`alternate`,
//! `reverse`, `alternate-reverse`). The easing applies within each step between
//! two keyframes, as in CSS. A stop that leaves a property out takes the
//! element's own value there, so `@keyframes pop { 40% { width: 150px } }`
//! grows from and back to whatever width the element has. An animation that
//! has finished leaves the element as its rules have it (CSS's default fill).
//! One animation an element; a comma-separated list plays its first.
//! It restarts when the declaration changes, so a script restarts one by
//! taking a class away and giving it back (`Ui.set_class`).

use std::collections::BTreeMap;

use sindri_core::{Easing, EntityId};
use weave::Keyframes;

use crate::transition::{Mixable, time};

/// What an `animation` declaration asks for.
#[derive(Clone, Debug, PartialEq)]
struct Animation {
    name: String,
    duration: f32,
    delay: f32,
    easing: Easing,
    /// How many times it plays; infinite for ever.
    iterations: f32,
    /// Whether it plays backward, and whether every other run turns round.
    reverse: bool,
    alternate: bool,
}

fn parse(value: &str) -> Option<Animation> {
    let first = value.split(',').next()?;
    let mut name = None;
    let mut times = Vec::new();
    let mut easing = Easing::Ease;
    let mut iterations = 1.0;
    let (mut reverse, mut alternate) = (false, false);
    for word in first.split_whitespace() {
        if let Some(seconds) = time(word) {
            times.push(seconds);
        } else if let Some(named) = Easing::named(word) {
            easing = named;
        } else if word == "infinite" {
            iterations = f32::INFINITY;
        } else if let Ok(count) = word.parse::<f32>() {
            iterations = count.max(0.0);
        } else if word == "alternate" {
            alternate = true;
        } else if word == "reverse" {
            reverse = true;
        } else if word == "alternate-reverse" {
            (alternate, reverse) = (true, true);
        } else if word != "normal" && name.is_none() {
            name = Some(word.to_owned());
        }
    }
    let duration = *times.first()?;
    let name = name?;
    (duration > 0.0).then(|| Animation {
        name,
        duration,
        delay: times.get(1).copied().unwrap_or(0.0),
        easing,
        iterations,
        reverse,
        alternate,
    })
}

/// One element's animation, and when it started.
#[derive(Clone, Debug)]
struct Run {
    written: String,
    animation: Animation,
    started: f32,
}

impl Run {
    fn ends(&self) -> f32 {
        self.started + self.animation.delay + self.animation.duration * self.animation.iterations
    }

    /// How far along its keyframes it is now, 0 to 1, or `None` while it
    /// waits out its delay or once it has finished.
    fn progress(&self, now: f32) -> Option<f32> {
        let elapsed = now - self.started - self.animation.delay;
        let runs = elapsed / self.animation.duration;
        if elapsed < 0.0 || runs >= self.animation.iterations {
            return None;
        }
        let run = runs.floor();
        let mut along = runs - run;
        // Whole numbers here are a run's count, far below 2^24.
        #[allow(clippy::cast_possible_truncation)]
        let odd = (run as i64) % 2 == 1;
        if self.animation.reverse != (self.animation.alternate && odd) {
            along = 1.0 - along;
        }
        Some(along)
    }
}

/// The animations playing, by stylesheet and element.
#[derive(Clone, Debug, Default)]
pub(crate) struct Animations {
    running: BTreeMap<(usize, EntityId), Run>,
}

impl Animations {
    /// Whether any is still playing at `now`.
    pub(crate) fn animating(&self, now: f32) -> bool {
        self.running.values().any(|run| run.ends() > now)
    }

    /// Lays the element's animation, if it has one, over its declarations:
    /// each property its keyframes name takes the value it has reached.
    pub(crate) fn play(
        &mut self,
        now: f32,
        key: (usize, EntityId),
        declarations: &mut BTreeMap<String, String>,
        keyframes: &BTreeMap<String, Keyframes>,
    ) {
        let Some(written) = declarations.get("animation").cloned() else {
            self.running.remove(&key);
            return;
        };
        if self
            .running
            .get(&key)
            .is_none_or(|run| run.written != written)
        {
            let Some(animation) = parse(&written) else {
                self.running.remove(&key);
                return;
            };
            self.running.insert(
                key,
                Run {
                    written,
                    animation,
                    started: now,
                },
            );
        }
        let run = &self.running[&key];
        let (Some(along), Some(frames)) = (run.progress(now), keyframes.get(&run.animation.name))
        else {
            return;
        };
        let properties: Vec<String> = frames
            .stops
            .iter()
            .flat_map(|(_, stop)| stop.keys().cloned())
            .collect();
        for property in properties {
            if let Some(value) = at(
                frames,
                &property,
                declarations.get(&property),
                along,
                run.animation.easing,
            ) {
                declarations.insert(property, value);
            }
        }
    }
}

/// `property`'s value `along` the keyframes: between the stops either side
/// that name it, the element's own value standing in at either end that does
/// not, eased within the step.
fn at(
    frames: &Keyframes,
    property: &str,
    own: Option<&String>,
    along: f32,
    easing: Easing,
) -> Option<String> {
    let own = own.and_then(|value| Mixable::parse(value));
    let mut stops: Vec<(f32, Mixable)> = frames
        .stops
        .iter()
        .filter_map(|(at, stop)| Some((*at, Mixable::parse(stop.get(property)?)?)))
        .collect();
    if let Some(own) = own {
        if stops.first().is_none_or(|(at, _)| *at > 0.0) {
            stops.insert(0, (0.0, own));
        }
        if stops.last().is_none_or(|(at, _)| *at < 1.0) {
            stops.push((1.0, own));
        }
    }
    let after = stops.iter().position(|(at, _)| *at >= along)?;
    let (end_at, end) = stops[after];
    if after == 0 {
        return Some(end.write());
    }
    let (start_at, start) = stops[after - 1];
    let span = end_at - start_at;
    let local = if span > 0.0 {
        (along - start_at) / span
    } else {
        1.0
    };
    start.mix(end, easing.apply(local)).map(Mixable::write)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pop() -> BTreeMap<String, Keyframes> {
        let sheet = weave::parse("@keyframes pop { 50% { width: 20px; } }").unwrap();
        sheet.keyframes
    }

    fn width(animations: &mut Animations, now: f32, animation: &str) -> String {
        let mut declarations = BTreeMap::from([
            ("width".to_owned(), "10px".to_owned()),
            ("animation".to_owned(), animation.to_owned()),
        ]);
        let entity = sindri_core::World::default().spawn(sindri_core::EntityData::default());
        animations.play(now, (0, entity), &mut declarations, &pop());
        declarations["width"].clone()
    }

    #[test]
    fn an_animation_plays_from_the_elements_own_value_and_back() {
        let mut animations = Animations::default();
        assert_eq!(width(&mut animations, 0.0, "pop 1s linear"), "10px");
        assert_eq!(width(&mut animations, 0.25, "pop 1s linear"), "15px");
        assert_eq!(width(&mut animations, 0.5, "pop 1s linear"), "20px");
        assert!(animations.animating(0.9));
        assert_eq!(
            width(&mut animations, 1.5, "pop 1s linear"),
            "10px",
            "finished, as its rules have it"
        );
        assert!(!animations.animating(1.5));
    }

    #[test]
    fn a_changed_declaration_starts_it_again_and_infinite_goes_on() {
        let mut animations = Animations::default();
        width(&mut animations, 0.0, "pop 1s linear");
        assert_eq!(
            width(&mut animations, 5.0, "pop 1s linear infinite"),
            "10px",
            "restarted at 5"
        );
        assert_eq!(
            width(&mut animations, 7.5, "pop 1s linear infinite"),
            "20px"
        );
        assert!(animations.animating(1_000.0));
    }

    #[test]
    fn animation_values_read_in_any_order() {
        let read = parse("pop 2 alternate 200ms ease-in 50ms").unwrap();
        assert_eq!(read.name, "pop");
        assert!((read.duration - 0.2).abs() < 1e-6 && (read.delay - 0.05).abs() < 1e-6);
        assert!((read.iterations - 2.0).abs() < 1e-6 && read.alternate && !read.reverse);
        assert!(parse("pop").is_none(), "a duration is needed");
    }
}
