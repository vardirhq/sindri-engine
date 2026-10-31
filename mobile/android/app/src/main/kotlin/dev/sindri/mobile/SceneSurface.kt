package dev.sindri.mobile

import android.content.Context
import android.view.Choreographer
import android.view.SurfaceHolder
import android.view.SurfaceView

/**
 * The view the engine draws the scene into.
 *
 * Compose owns everything over it; this owns only the surface's life: it is
 * handed to the native host when it has a size, followed when it resizes, and
 * let go before Android destroys it, which happens whenever the app leaves the
 * screen. Frames are drawn once per display refresh while it is attached and
 * [running].
 */
class SceneSurface(
    context: Context,
    private val onError: (String) -> Unit,
) : SurfaceView(context), SurfaceHolder.Callback, Choreographer.FrameCallback {

    private var attached = false
    private var scheduled = false

    /** Whether frames are drawn. Off while the activity is not resumed. */
    var running = false
        set(value) {
            field = value
            schedule()
        }

    init {
        holder.addCallback(this)
    }

    override fun surfaceCreated(holder: SurfaceHolder) = Unit

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        val error = if (attached) {
            NativeBridge.resize(width, height)
        } else {
            NativeBridge.attach(holder.surface, width, height)
        }
        if (error != null) {
            onError(error)
            return
        }
        attached = true
        schedule()
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        attached = false
        NativeBridge.detach()?.let(onError)
    }

    override fun doFrame(frameTimeNanos: Long) {
        scheduled = false
        if (!attached || !running) return
        val error = NativeBridge.draw()
        if (error != null) {
            onError(error)
            return
        }
        schedule()
    }

    private fun schedule() {
        if (scheduled || !attached || !running) return
        scheduled = true
        Choreographer.getInstance().postFrameCallback(this)
    }
}
