package dev.sindri.mobile

import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.MessageDigest

/** Replace only the exact, untouched old village, never an edited scene or recovery. */
internal object BundledVillageUpgrade {
    private const val OLD_VILLAGE_SHA256 = "5c175266fa04448076d9d6a3810c5fd65602a33fb6bf799797756cc56d8775fd"

    /**
     * The scripts that shipped beside that village, by their path among the
     * project's assets. The bundled village expects them as they are bundled
     * now: Wick's art is on his body, and a gloom's smoke and surge are parts
     * of it that `gloom.decay` plays by name.
     */
    private val OLD_SCRIPTS = mapOf(
        "scripts/wick.decay" to "34f04ef14aee7f31efd63f335275c2acebcac476e6fd59aa649d756fba26ecd5",
        "scripts/gloom.decay" to "d44fb8655a098ed5ddad05942cc9e8c50f0f45c3c56900741aeecd38dfb6088f",
    )

    /**
     * Replaces the old village under [assets], and the untouched scripts that
     * shipped with it, with the bundled ones [bundled] reads by path. The
     * scripts go first and the village only once every one is the bundled
     * one; a phone stopped between the two still plays, `wick.decay` reading
     * either layout and a gloom's surge merely staying dark.
     */
    fun apply(assets: File, recovery: File, bundled: (String) -> ByteArray) =
        apply(assets, recovery, OLD_VILLAGE_SHA256, OLD_SCRIPTS, bundled)

    internal fun apply(assets: File, recovery: File, oldVillage: String, oldScripts: Map<String, String>, bundled: (String) -> ByteArray) {
        val scene = File(assets, "village.scene")
        if (recovery.exists() || !scene.isFile || sha256(scene.readBytes()) != oldVillage) return
        for ((path, old) in oldScripts) replaceIfUntouched(File(assets, path), recovery, old) { bundled(path) }
        val current = oldScripts.keys.all { path -> File(assets, path).let { it.isFile && it.readBytes().contentEquals(bundled(path)) } }
        if (current) replaceIfUntouched(scene, recovery, oldVillage) { bundled("village.scene") }
    }

    internal fun sha256(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256")
        .digest(bytes).joinToString("") { "%02x".format(it) }

    internal fun replaceIfUntouched(scene: File, recovery: File, oldHash: String, bundled: () -> ByteArray) {
        if (recovery.exists() || !scene.isFile || sha256(scene.readBytes()) != oldHash) return
        val bytes = bundled()
        val temporary = Files.createTempFile(scene.parentFile.toPath(), ".village-upgrade-", ".tmp")
        try {
            temporary.toFile().outputStream().use { output ->
                output.write(bytes)
                output.fd.sync()
            }
            Files.move(temporary, scene.toPath(), StandardCopyOption.ATOMIC_MOVE, StandardCopyOption.REPLACE_EXISTING)
        } finally {
            Files.deleteIfExists(temporary)
        }
    }
}
