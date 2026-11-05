package dev.sindri.mobile

import android.graphics.BitmapFactory
import java.io.File
import java.io.InputStream
import java.util.UUID

/** Imported PNGs are copies inside the project, never replacements. */
object ImportedImages {
    fun list(project: File): List<AssetItem> {
        val folder = project.resolve("assets/textures")
        return folder.listFiles().orEmpty().filter { it.isFile && it.extension == "png" }
            .sortedBy { it.name.lowercase() }
            .map { AssetItem(it.nameWithoutExtension, "textures/${it.name}", image = ProjectImage(it.absolutePath)) }
    }

    fun import(project: File, input: InputStream): File {
        val folder = project.resolve("assets/textures")
        check(folder.isDirectory || folder.mkdirs()) { "Could not create the textures folder." }
        val id = UUID.randomUUID().toString()
        val file = folder.resolve("image-${id.take(8)}.png")
        val temporary = folder.resolve(".import-$id.tmp")
        check(temporary.createNewFile()) { "Could not create the image file." }
        try {
            input.use { source -> temporary.outputStream().use { output ->
                val buffer = ByteArray(8192)
                var total = 0L
                while (true) {
                    val count = source.read(buffer)
                    if (count < 0) break
                    total += count
                    require(total <= 32L * 1024 * 1024) { "The image is larger than 32 MB." }
                    output.write(buffer, 0, count)
                }
            } }
            val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
            BitmapFactory.decodeFile(temporary.path, options)
            require(options.outMimeType == "image/png" && options.outWidth > 0 && options.outHeight > 0) { "Choose a valid PNG image." }
            check(!file.exists() && temporary.renameTo(file)) { "Could not finish importing the image." }
            return file
        } catch (error: Exception) {
            temporary.delete()
            throw error
        }
    }
}
