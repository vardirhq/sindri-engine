package dev.sindri.mobile

import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.MessageDigest

/** Replace only the exact, untouched old village, never an edited scene or recovery. */
internal object BundledVillageUpgrade {
    private const val OLD_VILLAGE_SHA256 = "5c175266fa04448076d9d6a3810c5fd65602a33fb6bf799797756cc56d8775fd"

    /** The Wick script that shipped beside that village. */
    private const val OLD_WICK_SHA256 = "34f04ef14aee7f31efd63f335275c2acebcac476e6fd59aa649d756fba26ecd5"

    /**
     * The bundled village's Wick carries his art on his body, which only the
     * bundled script expects, so the village is replaced only once the script
     * is the bundled one. The script goes first: it also reads the old layout,
     * so a phone stopped between the two still plays.
     */
    fun apply(scene: File, script: File, recovery: File, bundledScene: () -> ByteArray, bundledScript: () -> ByteArray) {
        if (recovery.exists() || !scene.isFile || sha256(scene.readBytes()) != OLD_VILLAGE_SHA256) return
        replaceIfUntouched(script, recovery, OLD_WICK_SHA256, bundledScript)
        if (!script.isFile || !script.readBytes().contentEquals(bundledScript())) return
        replaceIfUntouched(scene, recovery, OLD_VILLAGE_SHA256, bundledScene)
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
