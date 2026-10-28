//! Entities, what is drawn on them, and the world they live in.

use super::{TypeEntry, call, value};

pub(super) const TYPES: &[TypeEntry] = &[
    TypeEntry {
        name: "World",
        text: "Finding, creating and removing the objects in a game. Every object in a scene (a player, an enemy, a button) is an entity.",
        members: &[
            call(
                "despawn",
                &["entity"],
                "Removes an object, and everything attached to it, from the game. Used when an enemy dies or a bullet hits something.",
            ),
            call(
                "exists",
                &["entity"],
                "Whether an object still exists. Check before using an object you stored earlier, since it may have been removed since.",
            ),
            call(
                "find",
                &["name"],
                "Finds an object by the name it was given in the scene, such as `World.find(\"Player\")`. Gives `null` if there is none.",
            ),
            call(
                "has_tag",
                &["entity", "tag"],
                "Whether an object has a tag, a label such as `\"enemy\"` given to it in the scene.",
            ),
            call(
                "is_active",
                &["entity"],
                "Whether an object is switched on. An object switched off, or inside one that is, is hidden and does nothing.",
            ),
            call(
                "nearest",
                &["tag", "position"],
                "The closest active object with an authored tag to a world-space Vec3 position, including parent transforms. Gives null if none has a usable transform. Equal distances keep world order.",
            ),
            call(
                "property_number",
                &["entity", "name", "fallback"],
                "Reads a number set up for another object's script, such as a bullet's damage, or gives `fallback` if there is none.",
            ),
            call(
                "send_signal",
                &["entity", "name", "value"],
                "Sends a number to another object under a name, for its script to collect with `take_signal`. Older style: events and calling another script's functions are clearer.",
            ),
            call(
                "set_active",
                &["entity", "on"],
                "Switches an object, and everything inside it, on or off. This is how menus and screens are shown and hidden.",
            ),
            call(
                "set_parent",
                &["entity", "parent"],
                "Attaches an object to another, so it moves, turns and scales with it. `null` detaches it.",
            ),
            call(
                "set_property",
                &["entity", "name", "value"],
                "Sets a starting value on a newly created object's script, before it starts. For example, a bullet's speed right after spawning it.",
            ),
            call(
                "set_shape_point",
                &["index", "x", "y"],
                "Moves one corner, numbered 0 to 7, of this object's custom polygon shape.",
            ),
            call(
                "spawn",
                &["prefab"],
                "Creates a new object from a prefab, a reusable template such as a bullet or an enemy, and gives it back so you can position it.",
            ),
            call(
                "spawn_child",
                &["prefab", "parent"],
                "Creates a new object from a prefab, already attached to a parent object.",
            ),
            call(
                "take_signal",
                &["name"],
                "Collects the numbers other scripts sent to this object under a name with `send_signal`, added together. Gives 0 when nothing arrived.",
            ),
            call(
                "with_tag",
                &["tag"],
                "Every switched-on object with a tag, as a list: `for enemy in World.with_tag(\"enemy\") { ... }`. Looks through the whole game, so call it once per frame, not in a loop.",
            ),
            call(
                "within_radius",
                &["tag", "position", "radius"],
                "A snapshot list of active tagged objects within an inclusive world-space radius of a Vec3 position, nearest first; ties keep world order. Skips unusable transforms. Empty when none match; over 8192 results is an error. Negative or NaN radii are errors; positive infinity searches the whole world.",
            ),
        ],
    },
    TypeEntry {
        name: "Entity",
        text: "An object in the game, such as a player, an enemy or a button. Scripts get these from calls like `World.find` and can store them, compare them and change their transform, sprite or shape.",
        members: &[
            value(
                "environment",
                "The authored scene environment on this object, if present.",
            ),
            value("light", "The object's 3D light, if it has one."),
            value("light2d", "The object's 2D light, if it has one."),
            value("shape", "The object's drawn shape, if it has one."),
            value("sprite", "The object's image (sprite), if it has one."),
            value("world_text", "The object's world text, if it has some."),
            value(
                "transform",
                "Where the object is, how it is turned and how big it is.",
            ),
            value(
                "ui_image",
                "The object's on-screen interface image, if it has one.",
            ),
        ],
    },
    TypeEntry {
        name: "Quat",
        text: "A copied quaternion value, with x/y/z/w components. Build with Quat(x: 0.0, y: 0.0, z: 0.0, w: 1.0), or copy transform.rotation. Change a local var's components, then assign the whole value; no quaternion arithmetic or interpolation methods are provided.",
        members: &[
            value("x", "The quaternion's X component."),
            value("y", "The quaternion's Y component."),
            value("z", "The quaternion's Z component."),
            value(
                "w",
                "The quaternion's scalar component; identity has w = 1 and x/y/z = 0.",
            ),
        ],
    },
    TypeEntry {
        name: "Transform",
        text: "Where an object is, how it is turned and how big it is. For an object attached to another, these are measured from that parent.",
        members: &[
            value(
                "rotation",
                "Parent-relative Quat in x/y/z/w order. Reads copy the stored rotation; whole writes normalize finite nonzero values atomically and preserve position, scale and Z lock. Stored components are readable; change components on a local var and assign it back. Zero/non-finite writes fail without mutation.",
            ),
            value(
                "forward",
                "Read-only unit world-space facing direction: -Z at identity. Parent rotation is included and scale is ignored. Copy it to move with `world_position += forward * speed * dt`.",
            ),
            value(
                "right",
                "Read-only unit world-space +X direction, with parent rotation included and scale ignored.",
            ),
            value(
                "up",
                "Read-only unit world-space +Y direction, with parent rotation included and scale ignored.",
            ),
            call(
                "look_at",
                &["target"],
                "Faces a world-space Vec3 target with Y up and zero world roll; preserves position and scale, including under a parent. Vertical targets use yaw zero. Coincident/non-finite targets are errors with no mutation.",
            ),
            call(
                "rotate_around",
                &["pivot", "axis", "radians"],
                "Rotates position and orientation around a world-space Vec3 pivot and nonzero Vec3 axis by radians. Normalizes the axis and preserves scale. Invalid input/output, a singular parent scale, or movement off a Z-locked layer is an error with no mutation.",
            ),
            value(
                "position",
                "Where the object is. Change it to move the object: `this.transform.position.x += speed * dt`. Measured from the parent if it has one.",
            ),
            value(
                "pitch",
                "How far a 3D object is tipped up (positive) or down from level, in radians, after its yaw. Changing it keeps its yaw and roll.",
            ),
            value(
                "roll",
                "How far a 3D object is turned about the way it faces, in radians, after its yaw and pitch. Changing it keeps its yaw and pitch.",
            ),
            value(
                "rotation_z",
                "How far the object is turned, in radians. A full turn is `TAU`. This is the turn of a flat object in a 2D game; it replaces any 3D turn.",
            ),
            value(
                "scale",
                "How big the object is on each axis; 1 is its normal size.",
            ),
            value(
                "world_position",
                "Where the object is in the world, even when it is attached to a parent. Use it to compare positions of objects in different places.",
            ),
            value(
                "yaw",
                "Which way a 3D object faces on the ground, in radians: a turn about the up (Y) axis. At zero it faces -Z, and positive turns it left. It moves along `Vec3(-sin(yaw), 0.0, -cos(yaw))`, and faces a point `d` away with `atan2(-d.x, -d.z)`. Changing it keeps its pitch and roll.",
            ),
        ],
    },
    TypeEntry {
        name: "Sprite",
        text: "A 2D image drawn in the game world, and how it is colored.",
        members: &[
            value(
                "color_multiply",
                "Another color the image is multiplied by, on top of `tint`.",
            ),
            value(
                "color_offset",
                "A color added on top of the image, which can make it brighter or flash white. Keep its `a` at 0, or the image's transparent edges become visible.",
            ),
            value(
                "layer",
                "Drawing order: higher numbers are drawn in front of lower ones.",
            ),
            value(
                "shader_params",
                "Four numbers the sprite's shader reads, as `x`, `y`, `z` and `w`. What each one does is up to the shader: how far a dissolve has got, how bright a flash is.",
            ),
            value(
                "tint",
                "A color the image is multiplied by. White leaves it unchanged; lowering `a` fades it out.",
            ),
        ],
    },
    TypeEntry {
        name: "Light",
        text: "A directional, point or spot 3D light. Writes validate the whole candidate before changing the payload. Missing components and invalid values are errors. Rotation/position come from the entity transform.",
        members: &[
            value(
                "color",
                "Linear RGB as a Color, whole or by channel. Alpha reads one and writes must keep it one; RGB is finite and nonnegative. Whole writes are atomic.",
            ),
            value(
                "intensity",
                "Finite nonnegative brightness; zero contributes no light.",
            ),
            value(
                "range",
                "Finite positive range in world units, independent of entity scale; used by points and spots.",
            ),
            value(
                "inner_angle",
                "Inner spotlight half-angle in radians, at least zero and strictly below outer_angle; use an order that keeps each write valid.",
            ),
            value(
                "outer_angle",
                "Outer spotlight half-angle in radians, above inner_angle and at most pi/2, with distinct f32 cosines.",
            ),
            value(
                "enabled",
                "Whether this light illuminates. Switching it off leaves its entity, scripts and children active.",
            ),
        ],
    },
    TypeEntry {
        name: "Light2d",
        text: "A light in a 2D scene: it brightens sprites, tiles and shapes near it when the scene's environment turns 2D lighting on. Switch one off with its object's `set_active`.",
        members: &[
            value(
                "color",
                "The light's color. Lowering `a` dims it, the way it fades a sprite.",
            ),
            value(
                "falloff",
                "How the light fades toward its edge: 1 fades evenly, higher numbers keep it bright near the middle and drop off sharply.",
            ),
            value(
                "height",
                "How high above the scene the light hangs. Only images with a normal map notice: low lights rake across their bumps, high ones light them evenly.",
            ),
            value("intensity", "How bright the light is; 0 is off."),
            value("radius", "How far the light reaches, in world units."),
        ],
    },
    TypeEntry {
        name: "WorldText",
        text: "Words placed in the game world, such as a damage number or a name over a character. Change the words with `Ui.set_text` and `Ui.set_number`, as for interface text.",
        members: &[
            value(
                "color",
                "The words' color. Lowering `a` fades them out; colored spans fade with it.",
            ),
            value(
                "font_size",
                "How tall a line of the words is, in world units.",
            ),
            value(
                "layer",
                "Drawing order: drawn over sprites in the same layer, under higher ones.",
            ),
            value(
                "visible",
                "How many letters show, for a typewriter reveal. Negative shows them all.",
            ),
        ],
    },
    TypeEntry {
        name: "ShaderParams",
        text: "The four numbers a sprite's shader reads. What each one means is up to the shader.",
        members: &[
            value("w", "The fourth number."),
            value("x", "The first number."),
            value("y", "The second number."),
            value("z", "The third number."),
        ],
    },
    TypeEntry {
        name: "Shape",
        text: "A shape drawn in the game world, such as a circle, polygon or ring, with an inside color and an outline.",
        members: &[
            value(
                "count",
                "How many sides a polygon has (3 is a triangle, 6 a hexagon), or how many cells a grid shape is across.",
            ),
            value(
                "dash_duty",
                "How much of each dash is drawn, from 0 to 1; the rest is the gap before the next. 0.5 draws dashes and gaps of equal length.",
            ),
            value(
                "dashes",
                "How many dashes the outline is broken into. 0 draws a solid line.",
            ),
            value("fill", "The color inside the shape."),
            value(
                "layer",
                "Drawing order: higher numbers are drawn in front of lower ones.",
            ),
            value("stroke", "The color of the shape's outline."),
            value(
                "stroke_width",
                "How thick the outline is, as a fraction of the shape's size.",
            ),
            value(
                "sweep_start",
                "Where the outline starts, as a fraction of the way round from the top.",
            ),
            value(
                "sweep_turns",
                "How much of the outline is drawn, from 0 to 1. Setting it to 0.5 draws half a ring, which makes cooldown and charge meters.",
            ),
        ],
    },
    TypeEntry {
        name: "UiImage",
        text: "An image in the interface drawn over the game, such as a health bar or an icon, rather than in the game world.",
        members: &[
            value(
                "layer",
                "Drawing order: higher numbers are drawn in front of lower ones.",
            ),
            value(
                "tint",
                "A color the image is multiplied by. White leaves it unchanged; lowering `a` fades it out.",
            ),
        ],
    },
    TypeEntry {
        name: "Effects",
        text: "Particle effects, such as sparks and explosions. The particles are only drawn; they are not objects and nothing can touch them.",
        members: &[
            call(
                "burst",
                &["entity"],
                "Plays the particle burst set up on an object, at that object's position. Gives back how many particles were made.",
            ),
            call(
                "burst_at",
                &["entity", "x", "y"],
                "Plays the particle burst set up on an object at another position, such as where an enemy just died.",
            ),
            call("live", &[], "How many particles are currently on screen."),
            call(
                "play",
                &["entity"],
                "Starts the particle emitter set up on an object. Gives back 1.",
            ),
            call(
                "stop",
                &["entity"],
                "Stops the particle emitter set up on an object; particles already out live out their life. Gives back 1.",
            ),
            call(
                "emit",
                &["entity", "count"],
                "Throws that many of an object's emitter particles at once, such as dust when landing. Gives back how many were made.",
            ),
        ],
    },
    super::physics::PHYSICS,
    super::character::CHARACTER_MOTION,
    TypeEntry {
        name: "RayHit2d",
        text: "A copied 2D ray hit snapshot. Null means no hit. Holds entity, world-space point and normal, and distance in world units. Copying or editing a snapshot never changes physics.",
        members: &[
            value("entity", "The entity owning the hit collider piece."),
            value("point", "The hit point in world coordinates, as Vec2."),
            value(
                "normal",
                "The world-space surface normal, as Vec2. Zero for a hit at distance zero.",
            ),
            value("distance", "Distance from the ray origin, in world units."),
        ],
    },
    TypeEntry {
        name: "Contact2d",
        text: "A copied solid solver contact from the last fixed step, relative to the queried body. Sensors are excluded. Multiple points/pieces may name one entity. Sleeping contacts retain support geometry and report zero new impulses/force. Editing a snapshot does not change physics.",
        members: &[
            value("entity", "The other entity touching the queried body."),
            value(
                "point",
                "World-space midpoint of the two surface anchors, as Vec2.",
            ),
            value(
                "normal",
                "World-space unit push direction towards the queried body, as Vec2.",
            ),
            value(
                "normal_impulse",
                "Nonnegative normal impulse from the last solve.",
            ),
            value(
                "tangent_impulse",
                "Signed friction impulse along Vec2(-normal.y, normal.x).",
            ),
            value(
                "force",
                "Total world impulse divided by the fixed step's seconds, as Vec2.",
            ),
        ],
    },
    TypeEntry {
        name: "Grid",
        text: "Grids of tiles, flat or stacked in layers like building blocks: what each cell holds, where objects stand on them, and paths across them.",
        members: &[
            call(
                "block",
                &["grid", "column", "row", "level"],
                "Which block is in a cell of a layered grid or a grid whose ground is a voxel world, by name, or `\"\"` if the cell is empty.",
            ),
            call(
                "can_reach",
                &["mover", "grid", "target"],
                "Whether a character on a grid can walk to where another object is.",
            ),
            call("columns", &["grid"], "How many columns a grid has."),
            call(
                "place",
                &["entity", "grid", "x", "y"],
                "Moves an object to a position on a grid, measured in cells. Fractions place it between cells.",
            ),
            call(
                "position_x",
                &["entity", "grid"],
                "Which column an object is at on a grid, with fractions between cells.",
            ),
            call(
                "position_y",
                &["entity", "grid"],
                "Which row an object is at on a grid, with fractions between cells.",
            ),
            call(
                "flooded",
                &["map", "column", "row"],
                "Whether a map column's existing surface is strictly below its flood level. Reads the same threshold the map draws; terrain and edits stay unchanged.",
            ),
            call(
                "set_flood",
                &["map", "level", "block"],
                "Sets a reversible water overlay on a voxel world viewed as a map, using a finite height and a block's top face. An empty block name clears it. Refused on block views.",
            ),
            call(
                "height",
                &["grid", "column", "row"],
                "How high the top of a voxel world's column is, as `Grid.surface` sees it, or `null` where nothing stands.",
            ),
            call("rows", &["grid"], "How many rows a grid has."),
            call(
                "set_block",
                &["grid", "column", "row", "level", "tile"],
                "Places a block, by name, in a cell of a layered grid or a grid whose ground is a voxel world. `\"\"` removes the block.",
            ),
            call(
                "set_terrain",
                &["grid", "column", "row", "terrain"],
                "Paints a terrain the map's sheet declares into a flat tile map's cell, choosing its tile and its neighbors' from which of them are the same terrain. `\"\"` empties the cell and redraws its neighbors.",
            ),
            call(
                "set_terrain_corner",
                &["grid", "column", "row", "terrain", "set"],
                "Sets or clears one corner of a `corners` terrain on a flat tile map, and redraws the up to four tiles that meet there. A corner is named by the grid line crossing it, so a map of 3 by 2 tiles has corners 0 to 3 across and 0 to 2 down. `set` is `true` to make the corner the terrain, `false` to give it back to the ground.",
            ),
            call(
                "set_tile",
                &["grid", "column", "row", "index"],
                "Changes a flat tile map's cell to one of its tiles, by number. A negative number empties the cell.",
            ),
            call(
                "step_toward",
                &["mover", "grid", "target"],
                "Moves a character one cell along the shortest walkable path towards a target. Gives back whether it moved.",
            ),
            call(
                "surface",
                &["grid", "column", "row"],
                "Which block is on top of a voxel world's column, by name, seen from straight above, or `\"\"` where nothing stands. What a map view of the world draws there.",
            ),
            call(
                "tagged",
                &["grid", "column", "row", "level", "tag"],
                "Whether the block in a cell has a label, such as `\"hot\"` on lava, given to it where the blocks are defined.",
            ),
            call(
                "tile",
                &["grid", "column", "row"],
                "Which tile is in a flat tile map's cell, by number, or -1 if it is empty or outside the map.",
            ),
            call(
                "walkable",
                &["grid", "x", "y"],
                "Whether a character could stand at a point on a grid. False for water, empty space or outside the grid.",
            ),
        ],
    },
];
