package dev.sindri.mobile

import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.MessageDigest

/** Replace only the exact, untouched old village, never an edited scene or recovery. */
internal object BundledVillageUpgrade {
    private const val OLD_VILLAGE_SHA256 = "5c175266fa04448076d9d6a3810c5fd65602a33fb6bf799797756cc56d8775fd"

    fun apply(scene: File, recovery: File, bundled: () -> ByteArray) =
        replaceIfUntouched(scene, recovery, OLD_VILLAGE_SHA256, bundled)

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
