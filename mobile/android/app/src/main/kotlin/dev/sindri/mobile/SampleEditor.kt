package dev.sindri.mobile

import androidx.compose.ui.graphics.Color

/**
 * The prototype's Lantern Crossing, as editor states: what each sheet shows
 * before it is fed by the engine. The design screenshots draw these, and the
 * app shows them for whatever is not wired to the engine yet.
 */
object SampleEditor {
    val base = EditorUi(projectTitle = "Lantern Crossing", sceneName = "world.scene")

    val prefabs = listOf(
        PrefabTile("Lamp post", Sheet.Objects, 0),
        PrefabTile("Bridge", Sheet.Dock, 0),
        PrefabTile("House", Sheet.Village, 0),
        PrefabTile("Cottage", Sheet.Village, 2),
        PrefabTile("Oak", Sheet.Village, 31),
        PrefabTile("Old oak", Sheet.Village, 30),
        PrefabTile("Bush", Sheet.Village, 13, 2.2f),
        PrefabTile("Flowers", Sheet.Village, 14, 2.2f),
        PrefabTile("Dead tree", Sheet.Objects, 3),
        PrefabTile("Rock", Sheet.Objects, 8, 1.4f),
        PrefabTile("Well", Sheet.Village, 39, 1.6f),
        PrefabTile("Wick", Sheet.Wick, 0, 1.6f),
    )

    val assets = prefabs.map { AssetItem(it.name, "prefabs/${it.name.lowercase().replace(' ', '_')}.prefab", it.sheet, it.cell, zoom = it.zoom) }

    val importFiles = listOf(
        ImportFile("gloom.png", "128 × 32 · 3 KB", true, Sheet.Gloom),
        ImportFile("villagers.png", "192 × 176 · 9 KB", true),
        ImportFile("wick-run.gif", "GIF isn't a supported texture yet", false),
    )

    val layers = listOf(
        LayerRow(Sym.grid_view, "Ground", "Tilemap · ground.png · 16 × 36", true),
        LayerRow(Sym.deployed_code, "Objects", "19 objects, sorted by height", true),
        LayerRow(Sym.sunny, "Lights", "2 lights · shown in play", true),
        LayerRow(Sym.web_asset, "Screen UI", "The HUD · edited in UI mode", true, locked = true),
    )

    val objects = listOf(
        TreeRow(0, Sym.account_tree, "World", chevron = Sym.expand_more),
        TreeRow(1, Sym.directions_walk, "Wick"),
        TreeRow(2, Sym.sunny, "Lantern light", "child"),
        TreeRow(1, Sym.emoji_objects, "Lamp post"),
        TreeRow(1, Sym.view_day, "Bridge"),
        TreeRow(1, Sym.folder, "Village", "3", chevron = Sym.chevron_right),
        TreeRow(1, Sym.folder, "Scenery", "13", chevron = Sym.chevron_right),
    )

    val scenes = listOf(
        SceneCard("world.scene", main = false, current = true, preview = true),
        SceneCard("title.scene", main = true, current = false, preview = false),
    )

    private val lampWarm = Color(0xFFFFD696)
    private val lampCool = Color(0xFFAAC8FF)
    private val lampEmber = Color(0xFFFF965A)

    fun lampInspector(details: Boolean = false, live: Boolean = false): InspectorUi {
        val place = SectionUi(
            Sym.open_with, "Place", "in tiles",
            listOf(
                RowUi.Stepper("Across", "X", Color(0xFFD6605C), "10.3"),
                RowUi.Stepper("Down", "Y", Color(0xFF7FB069), "22.3"),
                RowUi.Slider("Turn", 0.5f, "0°"),
                RowUi.Slider("Size", 1.5f / 4.5f, "2.0×"),
            ),
        )
        val light = SectionUi(
            Sym.sunny, "Light 2D", "when lit",
            listOf(
                RowUi.Slider("Radius", 3.5f / 7f, "4.5 tiles"),
                RowUi.Slider("Brightness", 1.2f / 3f, "1.2"),
                RowUi.Swatches("Color", listOf(lampWarm, lampCool, lampEmber), 0),
            ),
        )
        val sprite = SectionUi(
            Sym.image, "Sprite", rows = listOf(
                RowUi.SpriteRef("Sprite", Sheet.Objects, 0, "objects.png", "64 × 64 · sprite 0"),
                RowUi.Toggle("Mirror", false),
            ),
        )
        val collider = SectionUi(
            Sym.crop_square, "Collider 2D", rows = listOf(
                RowUi.Segment("Shape", listOf("Circle", "Box"), 0),
                RowUi.Toggle("Sensor (doesn’t block)", true),
                RowUi.Slider("Touch reach", 0.7f / 2.5f, "1.2 tiles"),
            ),
        )
        val behaviour = SectionUi(
            Sym.conversion_path, "Behaviour",
            rows = listOf(RowUi.Behaviour("When Wick touches this, light this lamp, shake the camera and open Bridge.")),
        )
        val detailRows = buildList<RowUi> {
            add(RowUi.More(if (details) "Hide details" else "Show all details", details))
            if (details) {
                add(
                    RowUi.Facts(
                        "Details",
                        listOf(
                            "Stable ID" to "lamp", "Prefab" to "prefabs/lamp_post.prefab", "Space" to "World",
                            "Z lock" to "on", "Draw order" to "by y, layer 0", "Position" to "330, 712 px",
                        ),
                    ),
                )
            }
        }
        val more = SectionUi(Sym.info, "Details", rows = detailRows)
        val sections = if (live) listOf(light) else listOf(place, light, sprite, collider, behaviour, more)
        return InspectorUi(Sym.emoji_objects, "Lamp post", if (live) "Live in this run" else "Prefab · Lamp post", true, sections, canAddComponent = !live)
    }

    val lampBehaviour = BehaviourUi(
        subject = "Lamp post",
        templates = listOf(
            BehaviourTemplate(Sym.touch_app, "Lights when touched", "Wick walks into it", true),
            BehaviourTemplate(Sym.front_hand, "Lights on Interact", "Wick presses a button nearby", false),
        ),
        rules = listOf(
            RuleUi(
                Sym.touch_app, "When Wick touches this",
                listOf(
                    ActionUi(Sym.emoji_objects, "Light this lamp"),
                    ActionUi(Sym.vibration, "Shake the camera", value = "0.18"),
                    ActionUi(Sym.door_front, "Open", target = "Bridge"),
                ),
            ),
        ),
        codeFile = "assets/scripts/lamp_post.decay",
        code = listOf(
            "script LampPost {",
            "    @export let opens: String = \"Bridge\";",
            "    @export let reach: number = 1.2;",
            "    var done: bool = false;",
            "    var wick: Entity = null;",
            "",
            "    fn start() {",
            "        this.wick = World.find(\"Wick\");",
            "    }",
            "",
            "    fn update(dt: number) {",
            "        if this.done { return; }",
            "        let dx = this.wick.transform.world_position.x - this.transform.world_position.x;",
            "        let dy = this.wick.transform.world_position.y - this.transform.world_position.y;",
            "        let near = sqrt(dx * dx + dy * dy) < this.reach;",
            "        if near {",
            "            this.done = true;",
            "            Animation.play(this.entity, \"lit\");",
            "            Camera.add_trauma(0.18);",
            "            emit LampLit(World.find(this.opens));",
            "        }",
            "    }",
            "}",
        ),
        codeNote = "Generated from 1 rule. Edit as code and the blocks become read-only for this script, so the two never disagree.",
    )

    val runEdits = listOf(
        RunEdit("Gloom", "Speed", "2.4", "1.6", keep = true),
        RunEdit("Lamp post", "Light radius", "4.5", "5.5", keep = false),
    )

    val lampSelection = SelectionUi(Sym.emoji_objects, "Lamp post", "Properties")
}
