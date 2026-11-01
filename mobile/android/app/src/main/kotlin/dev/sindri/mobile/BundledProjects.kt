package dev.sindri.mobile

import android.content.Context
import android.content.res.AssetManager
import java.io.File

/**
 * The sample projects shipped inside the APK, installed as app-owned copies.
 *
 * A project is an ordinary directory in the app's own storage, which the engine
 * reads and saves like any other. It is the app-specific external directory
 * (`Android/data/dev.sindri.mobile/files`) rather than internal storage, so a
 * saved project can be pulled to a computer with `adb pull` and opened in the
 * desktop editor; it is still the app's, removed with it and private to it on
 * current Android. Document-provider access to projects elsewhere on the phone
 * is a later, separate investigation (see `mobile/README.md`).
 */
object BundledProjects {
    private const val ROOT = "projects"
    private const val MARKER = ".installed-from"

    /**
     * The installed copy of the bundled project [name], copied out of the APK
     * the first time it is asked for.
     *
     * Never copied over once it exists: the copy is the person's project, saved
     * into, and an app update must not replace their edits with the bundled
     * original. The marker records which version it was installed from.
     */
    fun install(context: Context, name: String): File {
        val base = context.getExternalFilesDir(null) ?: context.filesDir
        val target = File(base, "$ROOT/$name")
        val marker = File(target, MARKER)
        if (marker.isFile) {
            return target
        }
        target.deleteRecursively()
        copy(context.assets, "$ROOT/$name", target)
        val version = context.packageManager.getPackageInfo(context.packageName, 0).versionName
        marker.writeText(version.orEmpty())
        return target
    }

    private fun copy(assets: AssetManager, path: String, target: File) {
        val children = assets.list(path).orEmpty()
        if (children.isEmpty()) {
            target.parentFile?.mkdirs()
            assets.open(path).use { input ->
                target.outputStream().use { output -> input.copyTo(output) }
            }
            return
        }
        target.mkdirs()
        for (child in children) {
            copy(assets, "$path/$child", File(target, child))
        }
    }
}
