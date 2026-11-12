package dev.sindri.mobile

import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.hypot
import kotlin.math.max
import kotlin.math.sin

/** A point or a size in the object's own units, with y growing upward as the engine has it. */
data class Vec2(val x: Double, val y: Double) {
    operator fun plus(other: Vec2) = Vec2(x + other.x, y + other.y)
    operator fun minus(other: Vec2) = Vec2(x - other.x, y - other.y)
    fun toList() = listOf(x, y)
}

/** How a part is drawn in the object workspace and where a finger finds it. */
sealed interface PartOutline {
    /** A rectangle [half] across each way, turned by [rotation] radians. */
    data class Box(val half: Vec2, val rotation: Double = 0.0) : PartOutline

    data class Circle(val radius: Double) : PartOutline

    data class Capsule(val radius: Double, val halfHeight: Double, val rotation: Double) : PartOutline

    /** An ellipse [half] across each way: a contact shadow. */
    data class Ellipse(val half: Vec2) : PartOutline

    /** A light: found by its center, drawn with the ring of its reach. */
    data class Reach(val radius: Double) : PartOutline
}

/** What a part's handle writes when it is dragged. */
sealed interface PartHandle {
    /** A size vector, from a corner: `size`, `shape_size`, `contact_size`. */
    data class Corner(val path: String, val size: Vec2, val whole: Boolean = true) : PartHandle

    /** A radius, from a point on its ring. */
    data class Ring(val path: String) : PartHandle
}

/** A field write: the component, the field's path in it, and its new value. */
data class PartEdit(val type: String, val path: String, val value: Any)

/**
 * One placeable part of an object, as the object workspace shows it: its art,
 * a light, an emitter, a contact shadow or one collider shape. It is found by
 * [key], moved by writing [offsetPath], and resized by its [handle].
 */
data class ObjectPart(
    val key: String,
    val label: String,
    val type: String,
    val offsetPath: String,
    val offset: Vec2,
    /** Where the part's outline is centered: the offset, moved by a sheet's anchor for art. */
    val center: Vec2,
    val outline: PartOutline,
    val handle: PartHandle? = null,
) {
    /** Where the handle sits, if the part has one. */
    fun handleAt(): Vec2? = when (val handle = handle) {
        is PartHandle.Corner -> center + Vec2(abs(handle.size.x) / 2, -abs(handle.size.y) / 2)
        is PartHandle.Ring -> (outline as? PartOutline.Reach)?.let { center + Vec2(it.radius, 0.0) }
        null -> null
    }

    /** The write that moves the part by [delta] from [start], its offset when the drag began. */
    fun moved(start: Vec2, delta: Vec2): PartEdit = PartEdit(type, offsetPath, (start + delta).toList())

    /** The write that puts the handle at [point]. */
    fun resized(point: Vec2): PartEdit? = when (val handle = handle) {
        is PartHandle.Corner -> {
            fun across(reach: Double, was: Double) = max(MINIMUM, 2 * abs(reach)) * (if (was < 0) -1.0 else 1.0)
            val size = Vec2(across(point.x - center.x, handle.size.x), across(point.y - center.y, handle.size.y))
            PartEdit(type, handle.path, size.toList())
        }
        is PartHandle.Ring -> PartEdit(type, handle.path, max(MINIMUM, hypot(point.x - center.x, point.y - center.y)))
        null -> null
    }

    /** Whether [point] lands on the part, [slop] units of leeway around it. */
    fun contains(point: Vec2, slop: Double): Boolean {
        val local = point - center
        return when (val outline = outline) {
            is PartOutline.Box -> {
                val turned = unturn(local, outline.rotation)
                abs(turned.x) <= outline.half.x + slop && abs(turned.y) <= outline.half.y + slop
            }
            is PartOutline.Circle -> hypot(local.x, local.y) <= outline.radius + slop
            is PartOutline.Capsule -> {
                val turned = unturn(local, outline.rotation)
                val along = (abs(turned.y) - outline.halfHeight).coerceAtLeast(0.0)
                hypot(turned.x, along) <= outline.radius + slop
            }
            is PartOutline.Ellipse -> {
                val a = outline.half.x + slop
                val b = outline.half.y + slop
                a > 0 && b > 0 && (local.x / a).let { it * it } + (local.y / b).let { it * it } <= 1.0
            }
            // A light's reach covers the whole object; only its center is the light.
            is PartOutline.Reach -> hypot(local.x, local.y) <= slop
        }
    }

    /** How much of the canvas the part covers, so the smallest of several under a finger wins. */
    val area: Double
        get() = when (val outline = outline) {
            is PartOutline.Box -> 4 * outline.half.x * outline.half.y
            is PartOutline.Circle -> Math.PI * outline.radius * outline.radius
            is PartOutline.Capsule -> Math.PI * outline.radius * outline.radius + 4 * outline.radius * outline.halfHeight
            is PartOutline.Ellipse -> Math.PI * outline.half.x * outline.half.y
            is PartOutline.Reach -> 0.0
        }

    /** The corners the workspace frames the object by. A light's reach is left out: it would shrink the art to a dot. */
    fun extent(): List<Vec2> = when (val outline = outline) {
        is PartOutline.Box -> boxCorners(outline.half, outline.rotation)
        is PartOutline.Circle -> boxCorners(Vec2(outline.radius, outline.radius), 0.0)
        is PartOutline.Capsule -> boxCorners(Vec2(outline.radius, outline.radius + outline.halfHeight), outline.rotation)
        is PartOutline.Ellipse -> boxCorners(outline.half, 0.0)
        is PartOutline.Reach -> listOf(center)
    }

    private fun boxCorners(half: Vec2, rotation: Double) =
        listOf(Vec2(-half.x, -half.y), Vec2(half.x, -half.y), Vec2(half.x, half.y), Vec2(-half.x, half.y))
            .map { center + turn(it, rotation) }

    companion object {
        /** The smallest a size or radius is dragged to, so a part cannot vanish under a finger. */
        const val MINIMUM = 0.05
    }
}

private fun turn(point: Vec2, radians: Double) =
    Vec2(point.x * cos(radians) - point.y * sin(radians), point.x * sin(radians) + point.y * cos(radians))

private fun unturn(point: Vec2, radians: Double) = turn(point, -radians)

/**
 * The parts of an object the workspace can place, read from the inspector's
 * components at their effective values. Everything is in the object's own
 * units, before its scale, which is where the engine places each part.
 */
object ObjectParts {
    const val SPRITE = "sindri.sprite"
    const val LIGHT = "sindri.light2d"
    const val PARTICLES = "sindri.particles"
    const val SHADOW = "sindri.shadow2d"
    const val COLLIDER = "sindri.physics2d.collider"

    fun of(inspector: Inspector): List<ObjectPart> = listOfNotNull(
        inspector.component(SPRITE)?.let { sprite(it, inspector.sprite?.anchor) },
        inspector.component(SHADOW)?.let(::shadow),
    ) + parts(inspector, PARTICLES, ::particles) + parts(inspector, LIGHT, ::light) +
        (inspector.component(COLLIDER)?.let(::collider) ?: emptyList())

    /** The kind's unnamed part, then each named one (`sindri.light2d#halo`), keyed and labelled by name. */
    private fun parts(inspector: Inspector, type: String, read: (Component) -> ObjectPart): List<ObjectPart> =
        inspector.components
            .filter { it.type == type || it.type.startsWith("$type#") }
            .sortedBy { it.type != type }
            .map { component ->
                val part = read(component)
                val name = component.type.substringAfter('#', "")
                if (name.isEmpty()) part else part.copy(key = "${part.key}#$name", label = "${part.label} · $name", type = component.type)
            }

    /** The smallest part under [point]. */
    fun at(parts: List<ObjectPart>, point: Vec2, slop: Double): ObjectPart? =
        parts.filter { it.contains(point, slop) }.minByOrNull { it.area }

    /** Whether [point] takes [part]'s handle. */
    fun onHandle(part: ObjectPart, point: Vec2, slop: Double): Boolean =
        part.handleAt()?.let { hypot(point.x - it.x, point.y - it.y) <= slop } == true

    /** The box the workspace frames: every part's extent and the object's origin. */
    fun bounds(parts: List<ObjectPart>): Pair<Vec2, Vec2> {
        val points = parts.flatMap { it.extent() } + Vec2(0.0, 0.0)
        val low = Vec2(points.minOf { it.x }, points.minOf { it.y })
        val high = Vec2(points.maxOf { it.x }, points.maxOf { it.y })
        return low to high
    }

    private fun vector(component: Component, path: String, fallback: Vec2): Vec2 {
        val numbers = (component.value(path) as? List<*>)?.mapNotNull { (it as? Number)?.toDouble() }
        return if (numbers != null && numbers.size >= 2) Vec2(numbers[0], numbers[1]) else fallback
    }

    private fun sprite(sprite: Component, anchor: List<Float>?): ObjectPart {
        val offset = vector(sprite, "offset", Vec2(0.0, 0.0))
        val size = vector(sprite, "size", Vec2(1.0, 1.0))
        // The sheet's anchor moves the picture within its own quad, so it scales with the size.
        val lift = Vec2((anchor?.getOrNull(0) ?: 0f) * size.x, (anchor?.getOrNull(1) ?: 0f) * size.y)
        return ObjectPart(
            "sprite", "Art", SPRITE, "offset", offset, offset + lift,
            PartOutline.Box(Vec2(abs(size.x) / 2, abs(size.y) / 2)),
            PartHandle.Corner("size", size),
        )
    }

    private fun light(light: Component): ObjectPart {
        val offset = vector(light, "offset", Vec2(0.0, 0.0))
        val radius = light.number("radius") ?: 4.0
        return ObjectPart("light", "Light", LIGHT, "offset", offset, offset, PartOutline.Reach(radius), PartHandle.Ring("radius"))
    }

    private fun particles(particles: Component): ObjectPart {
        val offset = vector(particles, "offset", Vec2(0.0, 0.0))
        val size = vector(particles, "shape_size", Vec2(0.0, 0.0))
        val outline = when (particles.value("shape")) {
            "circle", "ring" -> PartOutline.Circle(abs(size.x) / 2)
            "box" -> PartOutline.Box(Vec2(abs(size.x) / 2, abs(size.y) / 2))
            else -> PartOutline.Circle(0.0)
        }
        val handle = if (particles.value("shape") == "box") PartHandle.Corner("shape_size", size) else null
        return ObjectPart("particles", "Particles", PARTICLES, "offset", offset, offset, outline, handle)
    }

    private fun shadow(shadow: Component): ObjectPart? {
        val size = vector(shadow, "contact_size", Vec2(0.0, 0.0))
        if (size.x <= 0 || size.y <= 0) return null
        val offset = vector(shadow, "contact_offset", Vec2(0.0, 0.0))
        return ObjectPart(
            "shadow", "Shadow", SHADOW, "contact_offset", offset, offset,
            PartOutline.Ellipse(Vec2(size.x / 2, size.y / 2)),
            PartHandle.Corner("contact_size", size),
        )
    }

    private fun collider(collider: Component): List<ObjectPart> {
        val pieces = collider.value("pieces") as? List<*> ?: return emptyList()
        return pieces.indices.map { index ->
            val prefix = "pieces.$index."
            val offset = vector(collider, prefix + "offset", Vec2(0.0, 0.0))
            val rotation = collider.number(prefix + "rotation") ?: 0.0
            val radius = collider.number(prefix + "shape.radius") ?: 0.5
            val outline = when (collider.value(prefix + "shape.shape")) {
                "circle" -> PartOutline.Circle(radius)
                "capsule" -> PartOutline.Capsule(radius, collider.number(prefix + "shape.half_height") ?: 0.5, rotation)
                else -> PartOutline.Box(vector(collider, prefix + "shape.half_extents", Vec2(0.5, 0.5)), rotation)
            }
            val label = if (pieces.size == 1) "Collider" else "Collider ${index + 1}"
            ObjectPart("collider.$index", label, COLLIDER, prefix + "offset", offset, offset, outline)
        }
    }

    /** The collider piece a part key names, or null for another kind of part. */
    fun colliderPiece(key: String): Int? = key.removePrefix("collider.").takeIf { key.startsWith("collider.") }?.toIntOrNull()

    /** A tolerance in object units for [pixels] at [unit] pixels a unit. */
    fun slop(pixels: Float, unit: Float): Double = (pixels / unit.coerceAtLeast(1e-3f)).toDouble()
}
