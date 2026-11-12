package dev.sindri.mobile

import java.nio.file.Files
import org.junit.Assert.assertEquals
import org.junit.Test

class BundledVillageUpgradeTest {
    @Test
    fun only_an_untouched_scene_without_pending_recovery_is_replaced() {
        val directory = Files.createTempDirectory("village-upgrade").toFile()
        try {
            val scene = directory.resolve("village.scene")
            val recovery = directory.resolve("recovery.json")
            val old = "original scene".toByteArray()
            val hash = BundledVillageUpgrade.sha256(old)
            scene.writeBytes(old)
            recovery.writeText("unsaved edits")
            BundledVillageUpgrade.replaceIfUntouched(scene, recovery, hash) { "updated".toByteArray() }
            assertEquals("original scene", scene.readText())
            recovery.delete()
            scene.writeText("user edited")
            BundledVillageUpgrade.replaceIfUntouched(scene, recovery, hash) { "updated".toByteArray() }
            assertEquals("user edited", scene.readText())
            scene.writeBytes(old)
            BundledVillageUpgrade.replaceIfUntouched(scene, recovery, hash) { "updated".toByteArray() }
            assertEquals("updated", scene.readText())
        } finally {
            directory.deleteRecursively()
        }
    }

    @Test
    fun the_village_is_replaced_only_once_its_scripts_are_the_bundled_ones() {
        val assets = Files.createTempDirectory("village-upgrade").toFile()
        try {
            val scene = assets.resolve("village.scene")
            val scripts = assets.resolve("scripts").apply { mkdirs() }
            val recovery = assets.resolve("recovery.json")
            val oldVillage = "old village".toByteArray()
            scene.writeBytes(oldVillage)
            scripts.resolve("wick.decay").writeText("old wick")
            scripts.resolve("gloom.decay").writeText("edited gloom")
            val old = mapOf(
                "scripts/wick.decay" to BundledVillageUpgrade.sha256("old wick".toByteArray()),
                "scripts/gloom.decay" to BundledVillageUpgrade.sha256("old gloom".toByteArray()),
            )
            val bundled = { path: String -> "new $path".toByteArray() }
            val hash = BundledVillageUpgrade.sha256(oldVillage)
            // An edited script is kept, and so is the village it belongs with.
            BundledVillageUpgrade.apply(assets, recovery, hash, old, bundled)
            assertEquals("new scripts/wick.decay", scripts.resolve("wick.decay").readText())
            assertEquals("edited gloom", scripts.resolve("gloom.decay").readText())
            assertEquals("old village", scene.readText())
            // With every script untouched, both scripts and then the village are replaced.
            scripts.resolve("gloom.decay").writeText("old gloom")
            BundledVillageUpgrade.apply(assets, recovery, hash, old, bundled)
            assertEquals("new scripts/gloom.decay", scripts.resolve("gloom.decay").readText())
            assertEquals("new village.scene", scene.readText())
        } finally {
            assets.deleteRecursively()
        }
    }
}
