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
    fun the_village_is_replaced_only_with_its_script() {
        val directory = Files.createTempDirectory("village-upgrade").toFile()
        try {
            val scene = directory.resolve("village.scene")
            val script = directory.resolve("wick.decay")
            val recovery = directory.resolve("recovery.json")
            // Not the shipped old village: nothing is touched.
            scene.writeText("some village")
            script.writeText("some script")
            BundledVillageUpgrade.apply(scene, script, recovery, { "new village".toByteArray() }, { "new script".toByteArray() })
            assertEquals("some village", scene.readText())
            assertEquals("some script", script.readText())
        } finally {
            directory.deleteRecursively()
        }
    }
}
