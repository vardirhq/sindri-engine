package dev.sindri.mobile

import android.content.Context
import android.content.res.AssetManager
import java.io.File

/**
 * The sample projects shipped inside the APK, installed as app-owned copies.
 *
 * A project is an ordinary directory in the app's private storage, which the
 * engine reads like any other. Document-provider access to projects elsewhere
 * on the phone is a later, separate investigation (see `mobile/README.md`).
 */
object BundledProjects {
    private const val ROOT = "projects"
    private const val MARKER = ".installed-from"

    /**
     * The installed copy of the bundled project [name], copied out of the APK
     * the first time and again whenever the app is updated.
     *
     * Nothing edits a project on the phone yet, so replacing it on update loses
     * nothing. Once editing lands, an update must not overwrite a project the
     * person has changed; this is where that decision will be made.
     */
    fun install(context: Context, name: String): File {
        val target = File(context.filesDir, "$ROOT/$name")
        val marker = File(target, MARKER)
        val stamp = context.packageManager
            .getPackageInfo(context.packageName, 0)
            .lastUpdateTime
            .toString()
        if (marker.isFile && marker.readText() == stamp) {
            return target
        }
        target.deleteRecursively()
        copy(context.assets, "$ROOT/$name", target)
        marker.writeText(stamp)
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
