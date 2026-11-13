package dev.sindri.mobile

import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.MessageDigest

/** Replace only an exact, untouched old village, never an edited scene or recovery. */
internal object BundledVillageUpgrade {
    /**
     * Every village that shipped before the bundled one: the first, with
     * Wick and the lantern split into objects for their parts, and the one
     * whose lantern still hung its two lights from it.
     */
    private val OLD_VILLAGES = setOf(
        "5c175266fa04448076d9d6a3810c5fd65602a33fb6bf799797756cc56d8775fd",
        "3f77381f7744d3c9af16afe5df1718e1787db790dd2b1f73cc6fd88c9b5bafc3",
    )

    /**
     * The scripts that shipped beside those villages, by their path among the
     * project's assets. The bundled village expects them as they are bundled
     * now: Wick's art is on his body, the lantern's beam is its own light,
     * and a gloom's smoke and surge are parts of it that `gloom.decay` plays
     * by name.
     */
    private val OLD_SCRIPTS = mapOf(
        "scripts/wick.decay" to setOf(
            "34f04ef14aee7f31efd63f335275c2acebcac476e6fd59aa649d756fba26ecd5",
            "c75395f0f60f357b17a5519b26895db6bc826a4a0cab372c4ed949115bbc3757",
        ),
        "scripts/gloom.decay" to setOf(
            "d44fb8655a098ed5ddad05942cc9e8c50f0f45c3c56900741aeecd38dfb6088f",
            "e7a8f7e77d0d0986fee7b63c0d0ce1a9d2143c1e53098e44c1e3c1686d500d54",
        ),
    )

    /**
     * Replaces the old village under [assets], and the untouched scripts that
     * shipped with it, with the bundled ones [bundled] reads by path. The
     * scripts go first and the village only once every one is the bundled
     * one; a phone stopped between the two still plays, `wick.decay` reading
     * either layout and a gloom's surge merely staying dark.
     */
    fun apply(assets: File, recovery: File, bundled: (String) -> ByteArray) =
        apply(assets, recovery, OLD_VILLAGES, OLD_SCRIPTS, bundled)

    internal fun apply(
        assets: File,
        recovery: File,
        oldVillages: Set<String>,
        oldScripts: Map<String, Set<String>>,
        bundled: (String) -> ByteArray,
    ) {
        val scene = File(assets, "village.scene")
        if (recovery.exists() || !scene.isFile || sha256(scene.readBytes()) !in oldVillages) return
        for ((path, old) in oldScripts) replaceIfUntouched(File(assets, path), recovery, old) { bundled(path) }
        val current = oldScripts.keys.all { path -> File(assets, path).let { it.isFile && it.readBytes().contentEquals(bundled(path)) } }
        if (current) replaceIfUntouched(scene, recovery, oldVillages) { bundled("village.scene") }
    }

    internal fun sha256(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256")
        .digest(bytes).joinToString("") { "%02x".format(it) }

    internal fun replaceIfUntouched(scene: File, recovery: File, oldHashes: Set<String>, bundled: () -> ByteArray) {
        if (recovery.exists() || !scene.isFile || sha256(scene.readBytes()) !in oldHashes) return
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
