package dev.sindri.mobile

import android.view.Surface

/**
 * The calls into `libsindri_mobile.so`, the `sindri-mobile` crate.
 *
 * Every call must be made on the main thread: the native host lives there and
 * nowhere else. Each returns `null` when it succeeded and a message when it did
 * not, which the caller shows.
 */
object NativeBridge {
    init {
        System.loadLibrary("sindri_mobile")
    }

    /** Opens the scene file [scene], such as `world.scene`, in the project directory [project]. */
    @JvmStatic external fun open(project: String, scene: String): String?

    /** Starts drawing into [surface], which is [width] by [height] pixels. */
    @JvmStatic external fun attach(surface: Surface, width: Int, height: Int): String?

    /** The attached surface changed size. */
    @JvmStatic external fun resize(width: Int, height: Int): String?

    /** The attached surface is about to be destroyed; the scene and GPU are kept. */
    @JvmStatic external fun detach(): String?

    /** Draws one frame, when there is a surface and a scene. */
    @JvmStatic external fun draw(): String?

    /** One line describing the scene and the GPU drawing it. */
    @JvmStatic external fun status(): String
}
