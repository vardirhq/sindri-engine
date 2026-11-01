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


    /** `scripts/lamp.decay` as visual Decay reads it: a value, two handlers, comments and nesting. */
    val lampScript = DecayScript(
        name = "scripts/lamp.decay",
        source = "script Lamp { ... }",
        containers = listOf(
            DecayContainer(
                "script", "Lamp",
                fields = listOf(
                    DecayField("glow", exported = true, mutable = false, type = "number", value = DecaySlot.Number(1.5, 40 to 43), note = "How bright it burns once lit."),
                    DecayField("lit", exported = false, mutable = true, type = "bool", value = DecaySlot.Bool(false, 60 to 65), note = null),
                ),
                handlers = listOf(
                    DecayHandler(
                        "start", "When it starts", event = true, params = emptyList(), note = null,
                        body = listOf(
                            DecayBlock.Comment("Dark until Wick reaches it."),
                            DecayBlock.Set(DecaySlot.Name("this.light.enabled"), DecaySlot.Bool(false, 120 to 125)),
                        ),
                    ),
                    DecayHandler(
                        "update", "Every frame", event = true, params = listOf("dt"), note = null,
                        body = listOf(
                            DecayBlock.If(
                                DecaySlot.Op("&&", DecaySlot.Unary("!", DecaySlot.Name("this.lit")), DecaySlot.Call("World.near", listOf(DecaySlot.Name("this"), DecaySlot.Text("wick", 200 to 206), DecaySlot.Number(1.2, 208 to 211)))),
                                then = listOf(
                                    DecayBlock.Set(DecaySlot.Name("this.lit"), DecaySlot.Bool(true, 230 to 234)),
                                    DecayBlock.Change(DecaySlot.Name("Game.lit"), "+=", DecaySlot.Number(1.0, 250 to 253)),
                                    DecayBlock.Do(DecaySlot.Call("Camera.shake", listOf(DecaySlot.Number(0.3, 270 to 273)))),
                                ),
                                otherwise = null,
                            ),
                            DecayBlock.Code("this.light.intensity = lerp(this.light.intensity, this.glow, dt * 4.0);"),
                        ),
                    ),
                ),
            ),
        ),
    )
}
