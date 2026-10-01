//! The person playing: keys, pads, pointer, fingers, and the camera and
//! screen they look through.

use super::{TypeEntry, call, value};

pub(super) const TYPES: &[TypeEntry] = &[
    TypeEntry {
        name: "Input",
        text: "The keyboard. Keys are named by position, such as `\"W\"`, `\"ArrowLeft\"` or `\"Space\"`, so controls work on any keyboard layout.",
        members: &[
            call(
                "axis",
                &["negative", "positive"],
                "-1, 0 or 1 from a pair of keys, for movement: `Input.axis(\"A\", \"D\")` is -1 holding A, 1 holding D and 0 for neither or both.",
            ),
            call("is_down", &["key"], "Whether a key is being held down."),
            call(
                "just_pressed",
                &["key"],
                "Whether a key was pressed this frame. True only once per press, however long it is held.",
            ),
            call(
                "just_released",
                &["key"],
                "Whether a key was let go this frame.",
            ),
        ],
    },
    TypeEntry {
        name: "Action",
        text: "What the player means rather than what they pressed: the input actions the scene declares, such as `\"jump\"` or `\"move\"`, each bound to keys, buttons and sticks that can be changed while the game runs. A binding is written as text: `\"key.Space\"`, two sources for an axis (`\"key.A/key.D\"`), or four for a direction (`\"key.W/key.S/key.A/key.D\"`). A name the scene does not declare is an error.",
        members: &[
            call("axis", &["name"], "An axis action's value, from -1 to 1."),
            call(
                "bindings",
                &["name"],
                "What an action is bound to now, as a list of binding texts.",
            ),
            call("held", &["name"], "Whether an action is being made now."),
            call(
                "last_pressed",
                &[],
                "The key, mouse button or controller button pressed this frame, as a source name such as `\"key.J\"`, or empty text: for a screen that waits for the key to rebind to.",
            ),
            call(
                "pressed",
                &["name"],
                "Whether an action started this frame.",
            ),
            call(
                "rebind",
                &["name", "index", "binding"],
                "Replaces an action's binding at `index` with a binding text, or adds one at the next index. A binding that cannot make the action's kind is refused.",
            ),
            call(
                "released",
                &["name"],
                "Whether an action stopped this frame.",
            ),
            call(
                "vector",
                &["name"],
                "A direction action's value, as a Vec2 with right and up positive.",
            ),
        ],
    },
    TypeEntry {
        name: "Gamepad",
        text: "Game controllers. Each player gets a number from 1 to 8 when they press a button on their controller; 0 means any controller.",
        members: &[
            call(
                "axis",
                &["slot", "axis"],
                "How far a stick or trigger is pushed. Sticks (`\"left_x\"`, `\"left_y\"`…) give -1 to 1, with right and down positive; triggers give 0 to 1.",
            ),
            call(
                "count",
                &[],
                "How many players have joined with a controller.",
            ),
            call(
                "is_connected",
                &["slot"],
                "Whether a player number has a connected controller.",
            ),
            call(
                "is_down",
                &["slot", "button"],
                "Whether a controller button is held. Buttons are named by position, so `\"south\"` is the bottom face button on every make of controller.",
            ),
            call(
                "joined",
                &[],
                "The player number of a controller that joined this frame, or 0. Use it to add a player.",
            ),
            call(
                "just_pressed",
                &["slot", "button"],
                "Whether a mouse button was pressed this frame.",
            ),
            call(
                "just_released",
                &["slot", "button"],
                "Whether a controller button was let go this frame.",
            ),
            call(
                "left",
                &[],
                "The player number of a controller that was unplugged this frame, or 0. Use it to remove a player.",
            ),
        ],
    },
    TypeEntry {
        name: "Pointer",
        text: "The mouse, or a finger on a touch screen, handled the same way so one game works on both.",
        members: &[
            value(
                "inside",
                "Whether the mouse is over the game or a finger is on the screen. Check this before trusting a position.",
            ),
            call(
                "is_down",
                &["button"],
                "Whether a mouse button, `\"Left\"`, `\"Middle\"` or `\"Right\"`, is held. A finger on the screen counts as `\"Left\"`.",
            ),
            call(
                "just_pressed",
                &["button"],
                "Whether a mouse button was pressed this frame.",
            ),
            call(
                "just_released",
                &["button"],
                "Whether a mouse button was let go this frame, or the last finger lifted.",
            ),
            value(
                "over_ui",
                "Whether the pointer is over part of the interface, such as a button. Check it so clicking a button does not also shoot or move in the game.",
            ),
            value(
                "overlay",
                "The pointer's position in screen units, where 0 is the centre of the screen and the screen is 2 units tall. The same on any screen size.",
            ),
            value(
                "overlay_x",
                "The pointer's sideways position in screen units, where 0 is the centre.",
            ),
            value(
                "overlay_y",
                "The pointer's up-down position in screen units, where 0 is the centre.",
            ),
            value(
                "position",
                "The pointer's position in pixels, from the top-left corner of the game's view.",
            ),
            value(
                "x",
                "The pointer's distance from the left edge of the game's view, in pixels.",
            ),
            value(
                "y",
                "The pointer's distance from the top edge of the game's view, in pixels.",
            ),
        ],
    },
    TypeEntry {
        name: "Touch",
        text: "Every finger on a touch screen, numbered from 0, for games that need more than one finger.",
        members: &[
            value("count", "How many fingers are on the screen."),
            call(
                "x",
                &["index"],
                "A finger's distance from the left edge of the game's view, in pixels.",
            ),
            call(
                "y",
                &["index"],
                "A finger's distance from the top edge of the game's view, in pixels.",
            ),
        ],
    },
    TypeEntry {
        name: "Gesture",
        text: "What the player did with the mouse or a finger: a tap, a hold, a drag or a pinch. Works the same for both.",
        members: &[
            value(
                "drag_x",
                "How far the drag moved sideways this frame. Add it to a camera's position to pan.",
            ),
            value("drag_y", "How far the drag moved up or down this frame."),
            value("dragging", "Whether the player is dragging."),
            value(
                "held",
                "Whether the player has pressed and held still long enough to count as a long press.",
            ),
            value("hold_x", "Where the long press is, sideways."),
            value("hold_y", "Where the long press is, up or down."),
            value(
                "pinch",
                "How much two fingers pinched this frame: above 1 when spreading apart, below 1 when closing, and 1 when not pinching. Multiply a zoom by it.",
            ),
            value("pinching", "Whether two fingers are pinching."),
            value("tap_x", "Where the tap was, sideways."),
            value("tap_y", "Where the tap was, up or down."),
            value(
                "tapped",
                "Whether the player tapped (a quick press without moving) this frame. Check this before reading `tap_x` and `tap_y`.",
            ),
        ],
    },
    TypeEntry {
        name: "Aim",
        text: "Which block in a 3D block world the mouse or finger is pointing at, for building and digging games.",
        members: &[
            value(
                "hit",
                "Whether the pointer is over a block. Always check this first: when it is false, every other `Aim` value is 0, which is a real position.",
            ),
            value(
                "place_x",
                "Where a new block would go if placed now: the column of the empty space next to the side being pointed at.",
            ),
            value("place_y", "The row where a new block would go."),
            value("place_z", "The height level where a new block would go."),
            value("x", "The column of the block being pointed at."),
            value("y", "The row of the block being pointed at."),
            value("z", "The height level of the block being pointed at."),
        ],
    },
    TypeEntry {
        name: "Stick",
        text: "A virtual joystick for touch screens: wherever a thumb lands becomes the centre, and dragging from there steers.",
        members: &[
            value(
                "anchor_x",
                "Where the thumb first landed, sideways. Use it to draw the joystick.",
            ),
            value("anchor_y", "Where the thumb first landed, up or down."),
            value(
                "direction",
                "Which way and how far the joystick is pushed, as a direction up to 1 long.",
            ),
            value(
                "held",
                "Whether a thumb is on the joystick, even if it is not pushing.",
            ),
            value(
                "x",
                "How far the joystick is pushed right, from -1 (fully left) to 1 (fully right).",
            ),
            value(
                "y",
                "How far the joystick is pushed down, from -1 (fully up) to 1 (fully down).",
            ),
        ],
    },
    TypeEntry {
        name: "Camera",
        text: "The game's camera: moving the view and changing its engine-owned follow, confinement, and shake behavior.",
        members: &[
            call(
                "add_trauma",
                &["amount"],
                "Adds impact trauma to the authored behavior camera. Bigger amounts shake harder, up to 1, and the authored shake fades by itself.",
            ),
            call(
                "impact",
                &["amount"],
                "Shakes the authored behavior camera at least this hard, up to 1: a smaller impact while a bigger one is still shaking changes nothing, so many small hits in a frame do not add up to the biggest shake.",
            ),
            call(
                "follow",
                &["target"],
                "Makes the authored behavior camera follow an entity at runtime.",
            ),
            call(
                "clear_follow",
                &[],
                "Stops the authored behavior camera following a target.",
            ),
            call(
                "follow_offset",
                &["x", "y", "z"],
                "Sets the world-space offset from the follow target used by the authored behavior camera.",
            ),
            call(
                "dead_zone",
                &["x", "y"],
                "Sets the follow dead-zone size for the authored behavior camera.",
            ),
            call(
                "smoothing",
                &["value"],
                "Sets how strongly the authored behavior camera smooths its follow movement.",
            ),
            call(
                "max_speed",
                &["value"],
                "Sets the maximum follow speed of the authored behavior camera.",
            ),
            call(
                "bounds",
                &["min_x", "min_y", "max_x", "max_y"],
                "Confines the authored behavior camera to the given world-space rectangle.",
            ),
            call(
                "clear_bounds",
                &[],
                "Removes runtime confinement from the authored behavior camera.",
            ),
            call(
                "shake",
                &["strength", "frequency", "decay"],
                "Changes the authored behavior camera's shake strength, frequency, and trauma decay.",
            ),
            value(
                "pan_x",
                "How far the camera view is moved sideways from where it was placed in the scene.",
            ),
            value(
                "pan_y",
                "How far the camera view is moved up or down from where it was placed in the scene.",
            ),
            value(
                "pan_z",
                "How far the camera view is moved forwards or backwards from where it was placed in the scene.",
            ),
        ],
    },
    TypeEntry {
        name: "Viewport",
        text: "The area of the screen the game is drawn in.",
        members: &[value(
            "aspect",
            "The game view's width divided by its height: above 1 for a wide screen, below 1 for a phone held upright.",
        )],
    },
    TypeEntry {
        name: "Time",
        text: "Time in the game. A game runs as a series of frames, many times a second.",
        members: &[
            value(
                "delta",
                "How many seconds the last frame took. Multiply speeds by it so movement is the same at any frame rate. The same as `update`'s `dt`.",
            ),
            value("elapsed", "How many seconds this script has been running."),
        ],
    },
];
