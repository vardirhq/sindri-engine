package dev.sindri.mobile

/** Selections as the native host reports them, for tests that run without it. */
object EngineSamples {
    /** A lamp as Lamplighter has one: a light, a mirrored sprite, a circle collider and a script. */
    val lamp = Inspector(
        name = "Lamp post",
        position = listOf(10.0, -4.0, 0.2),
        id = "lamp-3",
        turn = 0.0,
        scale = listOf(-1.5, 1.5, 1.0),
        tile = 1.0,
        sprite = SpriteRegion("textures/objects.png", "lamp-post-lit", listOf(1f / 11, 0f, 1f / 11, 1f)),
        components = listOf(
            Component(
                "sindri.light2d", "Light 2D", emptyList(),
                values = mapOf("enabled" to true, "color" to listOf(1.0, 0.84, 0.59, 1.0), "intensity" to 1.2, "radius" to 4.5, "shadows" to false),
                meanings = mapOf("radius" to Meaning("range", min = 0.01), "intensity" to Meaning("range", min = 0.0), "color" to Meaning("color")),
            ),
            Component("sindri.sprite", "Sprite", emptyList(), values = mapOf("texture" to "textures/objects.png#lamp-post-lit", "layer" to 3)),
            Component(
                "sindri.physics2d.collider", "Collider 2D", emptyList(),
                values = mapOf("pieces" to listOf(mapOf("sensor" to true, "shape" to mapOf("shape" to "circle", "radius" to 1.2)))),
            ),
            Component("sindri.script", "Script", emptyList(), values = mapOf("script" to "Lamp", "source" to "scripts/lamp.decay")),
            Component("sindri.camera.behavior", "Camera Behaviour", emptyList(), values = mapOf("smoothing" to 0.25, "follow" to true)),
        ),
    )

}
