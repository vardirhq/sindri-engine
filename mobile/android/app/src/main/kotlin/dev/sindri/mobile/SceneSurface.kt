package dev.sindri.mobile

import android.annotation.SuppressLint
import android.content.Context
import android.view.Choreographer
import android.view.GestureDetector
import android.view.MotionEvent
import android.view.ScaleGestureDetector
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.ViewConfiguration
import kotlin.math.hypot

/**
 * The view the engine draws the scene into, and the gestures on it.
 *
 * Compose owns everything over it; this owns the surface's life and the touch
 * vocabulary of the canvas:
 *
 * - **Tap** selects what is under the finger, or clears the selection.
 * - **Drag** moves the selection when it starts on it, and pans otherwise, so
 *   nothing moves that was not chosen first. One drag is one undo step.
 * - **Pinch** zooms about the fingers.
 *
 * Every gesture calls [onChanged] so the overlay can refresh from the host,
 * with `true` when a finger lifts, which is when a move is finished and saved.
 */
class SceneSurface(
    context: Context,
    private val onError: (String) -> Unit,
    private val onChanged: (settled: Boolean) -> Unit,
) : SurfaceView(context), SurfaceHolder.Callback, Choreographer.FrameCallback {

    private var attached = false
    private var scheduled = false

    /** Where the current finger landed, until it has moved past the touch slop. */
    private var landed: Pair<Float, Float>? = null
    private var dragging = false
    private val slop = ViewConfiguration.get(context).scaledTouchSlop.toFloat()

    private val taps = GestureDetector(context, object : GestureDetector.SimpleOnGestureListener() {
        override fun onSingleTapUp(event: MotionEvent): Boolean {
            report(NativeBridge.command("tap", event.x, event.y))
            return true
        }
    })

    private val pinches = ScaleGestureDetector(context, object : ScaleGestureDetector.SimpleOnScaleGestureListener() {
        override fun onScaleBegin(detector: ScaleGestureDetector): Boolean {
            // A second finger ends whatever the first was doing.
            endDrag()
            return true
        }

        override fun onScale(detector: ScaleGestureDetector): Boolean {
            report(NativeBridge.command("zoom", detector.scaleFactor, detector.focusX, detector.focusY))
            return true
        }
    })

    /** Whether frames are drawn. Off while the activity is not resumed. */
    var running = false
        set(value) {
            field = value
            schedule()
        }

    init {
        holder.addCallback(this)
        isFocusable = true
        contentDescription = "Scene canvas. Tap to select, drag to move or pan, pinch to zoom."
    }

    @SuppressLint("ClickableViewAccessibility") // Taps go through GestureDetector.
    override fun onTouchEvent(event: MotionEvent): Boolean {
        pinches.onTouchEvent(event)
        taps.onTouchEvent(event)
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                landed = event.x to event.y
                dragging = false
                report(NativeBridge.command("press", event.x, event.y))
            }
            MotionEvent.ACTION_MOVE -> if (!pinches.isInProgress && event.pointerCount == 1) {
                val start = landed
                if (!dragging && start != null && hypot(event.x - start.first, event.y - start.second) > slop) {
                    dragging = true
                }
                if (dragging) report(NativeBridge.command("drag", event.x, event.y))
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> endDrag()
        }
        return true
    }

    private fun endDrag() {
        if (landed == null) return
        landed = null
        dragging = false
        report(NativeBridge.command("release"), settled = true)
    }

    private fun report(error: String?, settled: Boolean = false) {
        if (error != null) onError(error)
        onChanged(settled)
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
        onChanged(false)
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
