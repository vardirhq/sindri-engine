//! Physics host reference, separate from other world services.
use super::{TypeEntry, call};

pub(super) const PHYSICS: TypeEntry = TypeEntry {
    name: "Physics",
    text: "2D physics: moving objects with speed and pushes, and finding out what this script's object bumped into.",
    members: &[
        call(
            "move_character",
            &["entity", "displacement", "snap"],
            "Queues finite world-space Vec2 displacement for an active authored Character 2D at the next fixed scene step. Later requests replace earlier ones, including before spawn synchronization; invalid input preserves the pending request. snap permits authored downward snapping; ascending requests never snap. Requires scene controller context, valid character settings, a transform and no competing rigid body. Scene synchronization validates exactly one solid collider before applying input. Gameplay owns speed, gravity and jumps. Results are available from character_motion after the fixed step; solver response and discrete sensor events follow at the next solve.",
        ),
        call(
            "character_motion",
            &["entity"],
            "A copied CharacterMotion2d from the last fixed controller pass, or null before first movement or for inactive/non-controller entities. Requires scene controller context; stale entity handles are errors. All scripts read the same completed pass while requests queue separately. Inactive/despawned ground, platform and collision references are filtered; grounded and ground_walkable become false when ground is filtered. Mutating returned fields/lists never changes the cached result. Settings, parenting and teleports invalidate the cache at synchronization.",
        ),
        call(
            "set_slider_position_motor",
            &[
                "joint",
                "target_distance",
                "stiffness",
                "damping",
                "max_force",
            ],
            "Selects a damped force-based position motor on an authored slider owner. Target is finite signed anchor separation along the first body's local axis in world units without transform scale. Stiffness, damping and force cap must be finite and non-negative. Enabled travel limits still bound motion; zero force disables drive. Requires physics; validates before mutation, preserves unknown fields/body motion and applies at next synchronization even before bodies exist. Settings survive suspension/rebuilds. set_slider_motor selects velocity mode again.",
        ),
        call(
            "set_hinge_position_motor",
            &[
                "joint",
                "target_angle",
                "stiffness",
                "damping",
                "max_torque",
            ],
            "Selects a force-based position motor on an authored hinge owner. Target is relative radians within [-pi, pi]; stiffness, damping and torque cap must be finite and non-negative. Zero torque disables drive. Requires physics; validates before mutation, preserves unknown fields and body motion, and applies at next synchronization even before bodies exist. Settings survive suspension and endpoint rebuilds. set_hinge_motor selects velocity mode again. Enabled angular limits still bound motion.",
        ),
        call(
            "create_slider_joint",
            &[
                "joint",
                "first",
                "second",
                "first_anchor",
                "second_anchor",
                "first_axis",
                "second_axis",
                "limits_enabled",
                "lower_distance",
                "upper_distance",
            ],
            "Authors an enabled slider on an existing owner with no authored 2D joint of any kind. Finite Vec2 anchors use body-local world units without transform scale; axes must be finite unit Vec2 vectors. Bounds must be finite even when disabled; enabled lower distance cannot exceed upper distance. Motor starts disabled and set_slider_motor can configure it before synchronization. Scoped handles become stable IDs or local prefab paths; null leaves an endpoint unbound and inactive targets suspend until active. Requires physics, works before bodies are built and creates its constraint at next fixed synchronization. Invalid arguments or existing joints fail before mutation. Keeps other owner components and body motion; legacy connections remain separate.",
        ),
        call(
            "create_spring_joint",
            &[
                "joint",
                "first",
                "second",
                "first_anchor",
                "second_anchor",
                "rest_length",
                "stiffness",
                "damping",
            ],
            "Authors an enabled spring on an existing owner with no authored 2D joint of any kind. Finite Vec2 anchors use body-local world units without transform scale; rest length must be finite and positive, stiffness/damping finite and non-negative. Scoped handles become stable IDs or local prefab paths; null leaves an endpoint unbound and inactive targets suspend until active. Requires physics, works before bodies are built and creates its constraint at next fixed synchronization. Invalid arguments or existing joints fail before mutation. Keeps other owner components and body motion; legacy connections remain separate. Existing set_spring controls can retune the new joint before synchronization.",
        ),
        call(
            "create_hinge_joint",
            &["joint", "first", "second", "first_anchor", "second_anchor"],
            "Authors an enabled hinge on an existing owner with no authored 2D joint of any kind. Anchors are finite body-local Vec2 distances without transform scale. Angular limits and motor start disabled; set_hinge_motor can drive the new joint. Scoped handles become stable IDs or local prefab paths; null leaves an endpoint unbound and inactive targets suspend until active. Requires physics, works before bodies are built and creates its constraint at next fixed synchronization. Invalid owners, references, identical endpoints, anchors or existing joints fail before mutation. Keeps other owner components and body motion; legacy connections remain separate.",
        ),
        call(
            "create_distance_joint",
            &["joint", "first", "second", "max_distance"],
            "Authors an enabled maximum-distance joint on an existing owner with no authored 2D joint of any kind. Scoped endpoint handles become stable IDs or local prefab paths; null leaves an endpoint unbound and inactive endpoints suspend until active. Requires physics and a finite positive length. Valid before bodies are built; the next fixed synchronization creates the owned solver constraint. Invalid owners, references, identical endpoints, lengths or existing joints fail before mutation. Keeps other owner components and body motion; legacy connect_distance constraints remain separate.",
        ),
        call(
            "remove_joint",
            &["joint"],
            "Removes the authored distance, hinge, slider or spring component from one owner; the next fixed synchronization releases its solver constraint. Keeps the owner, its other components and endpoint bodies/motion. Legacy connect_distance constraints are unaffected. Works before bodies are built or while suspended. Requires physics and exactly one valid authored joint; missing, conflicting or malformed owners fail before mutation. Later joint controls fail until a joint is authored again.",
        ),
        call(
            "set_joint_endpoints",
            &["joint", "first", "second"],
            "Retargets an authored 2D distance, hinge, slider or spring at the next fixed synchronization. Accepts entity handles in the owner's scene or runtime prefab; null clears either endpoint. Stores stable scene IDs or canonical prefab paths, never runtime handles. Inactive endpoints may be authored and suspend until active. Stale, unstable, out-of-scope or identical endpoints fail before mutation. Preserves enabled state, settings, unknown fields and body motion. Requires physics and exactly one valid authored joint; works before bodies are built.",
        ),
        call(
            "joint_enabled",
            &["joint"],
            "Reads the authored enabled flag of one 2D joint owner, including before synchronization. True does not guarantee live endpoints; inactive or missing endpoints still suspend the constraint. Requires physics and exactly one valid authored joint.",
        ),
        call(
            "set_joint_enabled",
            &["joint", "enabled"],
            "Suspends or reconnects an authored distance, hinge, slider or spring at the next fixed synchronization. Keeps the owner, endpoints, settings and body motion; preserves unknown fields. Validates before mutation, including in the spawn window. Requires physics and exactly one valid authored joint.",
        ),
        call(
            "set_distance",
            &["joint", "max_distance"],
            "Tunes an authored 2D maximum-distance constraint for the next fixed synchronization. Distance must be finite and positive. Works while suspended or before endpoints are built; preserves unknown fields and body motion. Requires physics and an authored distance joint.",
        ),
        call(
            "contacts",
            &["entity"],
            "Copied Contact2d solid solver contacts from the last fixed step, ordered by other entity, point, normal and impulse. Normal points towards the queried body. Sensors are excluded; sleeping contacts report zero new impulses/force. Empty before simulation/spawn synchronization. Missing bodies/physics fail. Inactive/despawned others are omitted; removal and teleports invalidate affected snapshots.",
        ),
        call(
            "drop_through",
            &["entity", "seconds"],
            "Ignores only one-way solid platforms for a finite non-negative simulation duration; zero cancels. Dynamic bodies keep the solver timer, including queued spawn-window requests. Authored Character 2D entities instead queue a replacement scene-controller timer for the next fixed pass; positive remainder covers that whole pass and decrements afterward. Controller drop removes one-way support/carry and snap while ordinary floors and sensors retain their policy. Pause retains timers. Requires the corresponding physics/controller host context; invalid values preserve the previous request. No authored state is changed.",
        ),
        call(
            "continuous_collision",
            &["entity"],
            "Whether this authored 2D body uses swept solid collision. Sensors remain discrete.",
        ),
        call(
            "set_continuous_collision",
            &["entity", "enabled"],
            "Enables swept solid collision on a dynamic 2D body, including before a spawned body is built. Keeps velocity and joints.",
        ),
        call(
            "apply_force",
            &["entity", "force"],
            "Adds a world-space Vec2 force for the next fixed step, then clears it. Repeated calls add; requires a dynamic body.",
        ),
        call(
            "apply_torque",
            &["entity", "torque"],
            "Adds torque for the next fixed step, then clears it. Positive turns counterclockwise; requires a dynamic body.",
        ),
        call(
            "angular_velocity",
            &["entity"],
            "Radians per second. Before a spawned body is built, reads its authored value or latest queued setter.",
        ),
        call(
            "set_angular_velocity",
            &["entity", "velocity"],
            "Sets radians per second on a dynamic or velocity-kinematic body, respecting rotation locks.",
        ),
        call(
            "apply_angular_impulse",
            &["entity", "impulse"],
            "Applies an immediate angular impulse to a dynamic body, independent of timestep.",
        ),
        call(
            "apply_impulse_at_point",
            &["entity", "impulse", "point"],
            "Applies a world-space Vec2 impulse at a world-space Vec2 point, translating and turning a dynamic body. All motion controls support the spawn window.",
        ),
        call(
            "apply_impulse",
            &["entity", "x", "y"],
            "Gives an object a sudden push in a direction. Heavier objects move less.",
        ),
        call(
            "collision_started",
            &[],
            "The objects that started touching this script's object since the last frame, as a list.",
        ),
        call(
            "collision_stopped",
            &[],
            "The objects that stopped touching this script's object since the last frame, as a list.",
        ),
        call(
            "set_slider_motor",
            &["joint", "velocity", "max_force"],
            "Selects velocity mode on an authored 2D slider owner and sets its relative translation speed in world units/second along the first local axis, with a finite non-negative force cap. Zero force disables the drive; zero speed with positive force brakes. Validates before modifying the runtime component; applies at next fixed synchronization and survives endpoint rebuilds. Needs physics and a slider component; preserves unknown fields.",
        ),
        call(
            "set_spring",
            &["joint", "rest_length", "stiffness", "damping"],
            "Tunes an authored 2D spring owner's positive rest length and non-negative force-based stiffness/damping. All values must be finite. Invalid calls leave the component unchanged; valid changes apply at the next fixed synchronization, including before endpoints are built, and survive rebuilds. Needs physics and a spring component; preserves unknown fields.",
        ),
        call(
            "set_hinge_motor",
            &["joint", "velocity", "max_torque"],
            "Selects velocity mode on an authored 2D hinge owner and sets its relative angular velocity target in radians/second and finite non-negative torque cap. Positive turns the second body counterclockwise relative to the first. Zero torque disables the motor (it coasts rather than brakes). Applied at the next fixed synchronization, including before endpoints are built; persists through rebuilds. Invalid values leave the component unchanged. Needs physics and a hinge component on the owner.",
        ),
        call(
            "connect_distance",
            &["first", "second", "max_distance"],
            "Ties two objects together like a rope: they can come closer, but never further apart than a distance.",
        ),
        call(
            "cast_box",
            &[
                "origin",
                "half_size",
                "rotation",
                "direction",
                "max_distance",
                "mask",
                "include_sensors",
                "exclude",
            ],
            "Like `cast_circle`, for a box `half_size` from its centre to each edge, turned by `rotation` radians.",
        ),
        call(
            "cast_circle",
            &[
                "origin",
                "radius",
                "direction",
                "max_distance",
                "mask",
                "include_sensors",
                "exclude",
            ],
            "Sweeps a circle from `origin` along `direction` and returns the first collider it would touch, as RayHit2d or null: a raycast with a size, for whether something fits through a gap. `point` is where they touch and `distance` how far the circle's centre travelled. Starting already overlapping gives distance 0 and normal Vec2(0, 0). The filter arguments are the raycast's.",
        ),
        call(
            "gravity",
            &[],
            "Which way is down, as a Vec2 in world units per second squared: the first active Physics 2D World's authored gravity, or the host's own when the scene authors none.",
        ),
        call(
            "set_gravity",
            &["gravity"],
            "Changes the first active Physics 2D World's authored gravity to a finite Vec2, keeping its layer names. The next fixed step uses it and the inspector shows it; every body feels it, scaled by its own gravity scale. Fails when the scene authors no active world settings to change.",
        ),
        call(
            "layer",
            &["name"],
            "The mask for one collision layer the scene's physics world names, such as `\"ground\"`, for a query's `mask` argument. A name the world does not give is an error.",
        ),
        call(
            "mask",
            &["names"],
            "The mask for several named collision layers at once, such as `[\"ground\", \"enemies\"]`.",
        ),
        call(
            "overlap_box",
            &[
                "center",
                "half_size",
                "rotation",
                "mask",
                "include_sensors",
                "exclude",
            ],
            "Like `overlap_circle`, for a box `half_size` from its centre to each edge, turned by `rotation` radians.",
        ),
        call(
            "overlap_circle",
            &["center", "radius", "mask", "include_sensors", "exclude"],
            "Every object with a collider inside a circle, as a list, each once: an area check for a blast, an aura or a pickup radius. The filter arguments are the raycast's: which layers, whether trigger areas count, and one object to leave out (or null). Ignores inactive objects.",
        ),
        call(
            "raycast",
            &[
                "origin",
                "direction",
                "max_distance",
                "mask",
                "include_sensors",
                "exclude",
            ],
            "The closest 2D collider hit, as RayHit2d or null. Normalizes a nonzero Vec2 direction; returns world-space point, normal and distance. The mask selects collider memberships; include_sensors opts into triggers; exclude skips all pieces of one entity (or null). Origin, direction and distance must be finite; distance non-negative; mask a whole u32. Inside hits have distance 0 and normal Vec2(0, 0). Queries synchronized geometry; ignores inactive/despawned entities. Ties prefer entity handle then piece order.",
        ),
        call(
            "sensor_entered",
            &[],
            "The objects that entered this script's trigger area since the last frame. A trigger area detects things without blocking them, like a pickup.",
        ),
        call(
            "sensor_exited",
            &[],
            "The objects that left this script's trigger area since the last frame.",
        ),
        call(
            "set_velocity",
            &["entity", "x", "y"],
            "Sets how fast, and which way, an object is moving.",
        ),
        call(
            "velocity_x",
            &["entity"],
            "How fast an object is moving sideways.",
        ),
        call(
            "velocity_y",
            &["entity"],
            "How fast an object is moving up or down.",
        ),
    ],
};
