//! Screens, sound, saves, randomness and the rest of a game around its world.

use super::{TypeEntry, call};

pub(super) const TYPES: &[TypeEntry] = &[
    TypeEntry {
        name: "Ui",
        text: "The interface drawn over the game: changing text and bars, and checking buttons and sliders.",
        members: &[
            call(
                "is_held",
                &["button"],
                "Whether a button is being held down right now.",
            ),
            call(
                "is_hovered",
                &["button"],
                "Whether the mouse or finger is over a button.",
            ),
            call(
                "position",
                &["element"],
                "Where an element was laid out on the screen, as a Vec2 in overlay units: 0 is the middle, and one unit is half the screen's height, up positive.",
            ),
            call(
                "size",
                &["element"],
                "How wide and tall an element was laid out, in the same units as `position`, including room a stylesheet's `flex-grow` gave it.",
            ),
            call(
                "is_pressed",
                &["button"],
                "Whether a button was clicked this frame: pressed and let go while on it.",
            ),
            call(
                "set_fill",
                &["bar", "amount"],
                "Shows part of an image, from 0 (none) to 1 (all). This is how health and progress bars work: `Ui.set_fill(bar, hp / max_hp)`.",
            ),
            call(
                "set_number",
                &["label", "value"],
                "Puts a number into a text element. Its text is set up in the scene with a `{}` where the number goes, such as `\"Score: {}\"`.",
            ),
            call(
                "set_numbers",
                &["label", "first", "second"],
                "Puts two numbers into a text element with two `{}` places, such as `\"{}/{}\"` for `45/100`.",
            ),
            call(
                "set_slider_value",
                &["slider", "value"],
                "Moves a slider to a value, kept within the slider's range.",
            ),
            call(
                "set_text",
                &["label", "text"],
                "Changes the words a text element shows.",
            ),
            call(
                "slider_changed",
                &["slider"],
                "Whether the player moved a slider this frame.",
            ),
            call("is_checked", &["toggle"], "Whether a toggle is checked."),
            call(
                "set_checked",
                &["toggle", "checked"],
                "Sets a toggle without emitting a user-change event.",
            ),
            call(
                "input_text",
                &["input"],
                "The committed text in a single-line input.",
            ),
            call(
                "set_input_text",
                &["input", "text"],
                "Writes text using the input's Unicode length limit.",
            ),
            call(
                "scroll_offset",
                &["scroll"],
                "Vertical scroll distance in overlay units.",
            ),
            call(
                "selected",
                &["dropdown"],
                "Which option a dropdown has chosen, counted from zero.",
            ),
            call(
                "set_selected",
                &["dropdown", "index"],
                "Chooses a dropdown's option without emitting a user-change event.",
            ),
            call(
                "is_open",
                &["dropdown"],
                "Whether a dropdown's list is open.",
            ),
            call(
                "caret",
                &["input"],
                "Where a text field's caret is, in characters.",
            ),
            call(
                "selection_start",
                &["input"],
                "Where a text field's selection begins, in characters; the caret when nothing is selected.",
            ),
            call(
                "selection_end",
                &["input"],
                "Where a text field's selection ends, in characters; the caret when nothing is selected.",
            ),
            call(
                "set_scroll_offset",
                &["scroll", "offset"],
                "Clamps the offset to the authored content height.",
            ),
            call(
                "changed",
                &["element"],
                "Whether user interaction changed a widget this step.",
            ),
            call(
                "submitted",
                &["input"],
                "Whether Enter submitted the focused input this step.",
            ),
            call(
                "is_focused",
                &["element"],
                "Whether this element has keyboard focus.",
            ),
            call("slider_value", &["slider"], "The value a slider is set to."),
        ],
    },
    TypeEntry {
        name: "Animation",
        text: "Playing an object's animations, such as walk or jump, set up for it in the scene.",
        members: &[
            call(
                "clip",
                &["entity"],
                "The name of the animation an object is playing.",
            ),
            call(
                "frame",
                &["entity"],
                "Which picture of its animation an object is showing, counting from the start of the animation.",
            ),
            call(
                "is_finished",
                &["entity"],
                "Whether an animation that plays once, such as an attack, has finished.",
            ),
            call(
                "play",
                &["entity", "clip"],
                "Plays one of an object's animations by name, such as `\"walk\"`. Calling it again with the animation already playing does nothing, so it is safe to call every frame.",
            ),
            call(
                "restart",
                &["entity"],
                "Starts an object's current animation again from the beginning.",
            ),
            call(
                "set_speed",
                &["entity", "speed"],
                "How fast an object's animations play: 1 is normal speed, 2 is twice as fast.",
            ),
            call("stop", &["entity"], "Stops an object's animation."),
        ],
    },
    TypeEntry {
        name: "Sequence",
        text: "Playing an object's sequences: choreography set up for it in the scene's Timeline, such as a title sliding in or a door opening, with named cues along the way.",
        members: &[
            call(
                "cued",
                &["entity", "cue"],
                "Whether an object's sequence reached the named cue on the last step, such as `\"landed\"`. True for one step only.",
            ),
            call(
                "is_finished",
                &["entity"],
                "Whether a sequence that does not loop has reached its end.",
            ),
            call(
                "play",
                &["entity", "sequence"],
                "Plays one of an object's sequences by name. Calling it again with the sequence already playing does nothing, so it is safe to call every frame.",
            ),
            call(
                "playing",
                &["entity"],
                "The name of the sequence an object is playing, or empty text.",
            ),
            call(
                "restart",
                &["entity"],
                "Starts an object's current sequence again from the beginning.",
            ),
            call(
                "set_speed",
                &["entity", "speed"],
                "How fast an object's sequences play: 1 is normal speed, 0 holds it where it is.",
            ),
            call(
                "stop",
                &["entity"],
                "Stops an object's sequence where it is.",
            ),
            call(
                "time",
                &["entity"],
                "How far into its sequence an object has got, in seconds.",
            ),
        ],
    },
    TypeEntry {
        name: "Audio",
        text: "Playing sound effects and music. Every sound goes through a bus — `\"effects\"`, `\"music\"`, or any other name — and every bus through `\"master\"`, so a settings screen can turn each down.",
        members: &[
            call(
                "loop",
                &["clip", "volume"],
                "Plays a sound over and over until stopped, such as music, at a volume from 0 (silent) to 1 (full), through the `\"music\"` bus.",
            ),
            call(
                "loop_on",
                &["bus", "clip", "volume"],
                "Plays a sound over and over through a bus you name.",
            ),
            call("pause_all", &[], "Pauses every sound that is playing."),
            call(
                "play",
                &["clip", "volume"],
                "Plays a sound once, at a volume from 0 (silent) to 1 (full), through the `\"effects\"` bus.",
            ),
            call(
                "play_on",
                &["bus", "clip", "volume"],
                "Plays a sound once through a bus you name.",
            ),
            call("resume_all", &[], "Continues every paused sound."),
            call(
                "set_volume",
                &["bus", "volume"],
                "Sets a bus's volume from 0 to 1, at once for every sound playing through it. `\"master\"` turns everything up or down.",
            ),
            call("stop_all", &[], "Stops every sound that is playing."),
            call(
                "volume",
                &["bus"],
                "A bus's volume as last set, or 1 for one never set.",
            ),
        ],
    },
    TypeEntry {
        name: "Scene",
        text: "Scenes are a game's separate places or screens, such as a menu, a level or a shop. This asks which one is playing and moves to another.",
        members: &[
            call("current", &[], "The name of the scene being played."),
            call(
                "go",
                &["name"],
                "Moves to another scene by name, at the end of this frame.",
            ),
        ],
    },
    TypeEntry {
        name: "Save",
        text: "Saving progress between play sessions: numbers and true/false values stored under names, such as a best score.",
        members: &[
            call("clear", &[], "Deletes everything saved."),
            call(
                "flag",
                &["key", "fallback"],
                "A saved true/false value, or `fallback` if nothing is saved under that name yet.",
            ),
            call("has", &["key"], "Whether anything is saved under a name."),
            call(
                "is_damaged",
                &[],
                "Whether saved progress existed but could not be read, so the game can tell the player before overwriting it.",
            ),
            call(
                "is_from_newer",
                &[],
                "Whether the save was made by a newer version of the game. Its values are not loaded.",
            ),
            call(
                "is_new",
                &[],
                "Whether nothing has been saved yet, as on the first time the game is played.",
            ),
            call(
                "number",
                &["key", "fallback"],
                "A saved number, or `fallback` if nothing is saved under that name yet: `Save.number(\"best\", 0.0)`.",
            ),
            call(
                "set_flag",
                &["key", "value"],
                "Saves a true/false value under a name.",
            ),
            call(
                "set_number",
                &["key", "value"],
                "Saves a number under a name.",
            ),
        ],
    },
    TypeEntry {
        name: "Random",
        text: "Random numbers. The same seed always gives the same numbers, so a run can be replayed.",
        members: &[
            call(
                "int",
                &["min", "max"],
                "A random whole number from `min` to `max`, both included: `Random.int(1.0, 6.0)` rolls a die.",
            ),
            call(
                "pick",
                &["group"],
                "A random object from a list, such as a random enemy to target. The list must not be empty.",
            ),
            call(
                "range",
                &["min", "max"],
                "A random number from `min` up to, but not including, `max`.",
            ),
            call(
                "seed",
                &["value"],
                "Starts the random numbers from a seed. The same seed always gives the same numbers afterwards.",
            ),
            call(
                "value",
                &[],
                "A random number from 0 up to, but not including, 1.",
            ),
        ],
    },
    TypeEntry {
        name: "Game",
        text: "Numbers stored under names that every script can read and write, such as a score. Declaring them with `state` is safer, because a misspelt name then becomes an error.",
        members: &[
            call(
                "get",
                &["name", "fallback"],
                "The number stored under a name, or `fallback` if nothing has been stored yet.",
            ),
            call(
                "set",
                &["name", "value"],
                "Stores a number under a name for any script to read.",
            ),
        ],
    },
    TypeEntry {
        name: "Profiles",
        text: "Reading settings from a profile, a data file shared across a project, such as a weapon's damage and fire rate.",
        members: &[
            call(
                "count",
                &["profile", "collection"],
                "How many items a list in the profile has.",
            ),
            call(
                "flag",
                &["profile", "key", "fallback"],
                "A true/false setting from the profile, or `fallback` if it is missing.",
            ),
            call(
                "flag_at",
                &["profile", "collection", "index", "key", "fallback"],
                "A true/false setting from one item in a list in the profile, or `fallback`.",
            ),
            call("kind", &["profile"], "What kind of profile it is."),
            call("name", &["profile"], "The profile's name."),
            call(
                "number",
                &["profile", "key", "fallback"],
                "A number setting from the profile, or `fallback` if it is missing: `Profiles.number(this.tuning, \"damage\", 1.0)`.",
            ),
            call(
                "number_at",
                &["profile", "collection", "index", "key", "fallback"],
                "A number setting from one item in a list in the profile, or `fallback`.",
            ),
            call(
                "text",
                &["profile", "key", "fallback"],
                "A text setting from the profile, or `fallback` if it is missing.",
            ),
            call(
                "text_at",
                &["profile", "collection", "index", "key", "fallback"],
                "A text setting from one item in a list in the profile, or `fallback`.",
            ),
        ],
    },
    TypeEntry {
        name: "Profile",
        text: "A profile: a data file of settings shared across a project. Chosen in the editor for a script's `@export` field and read with `Profiles`.",
        members: &[],
    },
];
