package dev.sindri.mobile

import android.content.Context
import android.graphics.BitmapFactory
import androidx.compose.foundation.Canvas
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.FilterQuality
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import kotlin.math.ceil
import kotlin.math.roundToInt

/**
 * A sprite sheet from the bundled sample project, cut into equal cells, as the
 * prototype's `SHEETS` table describes Lamplighter's textures.
 */
enum class Sheet(val file: String, val cellWidth: Int, val cellHeight: Int) {
    Wick("wick.png", 48, 44),
    Objects("objects.png", 64, 64),
    Village("village.png", 112, 112),
    Gloom("gloom.png", 32, 32),
    Dock("dock.png", 24, 64),
    Ground("ground.png", 64, 64),
    /** One 16 × 16 tile of the ground tileset, as the paint brushes show it. */
    GroundTile("ground.png", 16, 16),
}

/**
 * Part of an image in a bundled project: its path among the app's assets and
 * the part of it to draw, as fractions of the image (left, top, width, height),
 * which is how the engine's sheets measure a sprite.
 */
data class ProjectImage(val path: String, val rect: List<Float> = listOf(0f, 0f, 1f, 1f))

/** Decoded images, kept for the life of the process: they are small and reused. */
private object SheetCache {
    private val loaded = mutableMapOf<String, ImageBitmap?>()

    fun get(context: Context, sheet: Sheet): ImageBitmap? = get(context, "projects/lamplighter/assets/textures/${sheet.file}")

    fun get(context: Context, path: String): ImageBitmap? = loaded.getOrPut(path) {
        runCatching {
            context.assets.open(path).use { BitmapFactory.decodeStream(it)?.asImageBitmap() }
        }.getOrNull()
    }
}

/**
 * Draws [image] as large as fits the bounds, standing on their bottom edge as
 * a sprite stands on the ground, pixel art unsmoothed. Nothing where the image
 * is not bundled.
 */
@Composable
fun ProjectSprite(image: ProjectImage, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val bitmap = remember(image.path) { SheetCache.get(context, image.path) } ?: return
    Canvas(modifier) {
        val (left, top, width, height) = image.rect
        val source = IntSize((width * bitmap.width).roundToInt().coerceAtLeast(1), (height * bitmap.height).roundToInt().coerceAtLeast(1))
        val scale = minOf(size.width / source.width, size.height / source.height)
        val drawn = IntSize((source.width * scale).roundToInt(), (source.height * scale).roundToInt())
        drawImage(
            bitmap,
            srcOffset = IntOffset((left * bitmap.width).roundToInt(), (top * bitmap.height).roundToInt()),
            srcSize = source,
            dstOffset = IntOffset(((size.width - drawn.width) / 2).roundToInt(), (size.height - drawn.height).roundToInt()),
            dstSize = drawn,
            filterQuality = FilterQuality.None,
        )
    }
}

/** Draws cell [cell] of [sheet] filling the modifier's bounds, pixel art unsmoothed. */
@Composable
fun Sprite(sheet: Sheet, cell: Int, modifier: Modifier = Modifier, row: Int = 0) {
    val context = LocalContext.current
    val image = remember(sheet) { SheetCache.get(context, sheet) } ?: return
    Canvas(modifier) {
        drawImage(
            image,
            srcOffset = IntOffset(cell * sheet.cellWidth, row * sheet.cellHeight),
            srcSize = IntSize(sheet.cellWidth, sheet.cellHeight),
            dstSize = IntSize(size.width.roundToInt(), size.height.roundToInt()),
            filterQuality = FilterQuality.None,
        )
    }
}

/** Tiles the whole of [sheet] across the bounds at [tile] pixels per repeat. */
@Composable
fun TiledSheet(sheet: Sheet, tile: Float, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val image = remember(sheet) { SheetCache.get(context, sheet) } ?: return
    Canvas(modifier) {
        val side = tile.roundToInt().coerceAtLeast(1)
        val across = ceil(size.width / side).toInt()
        val down = ceil(size.height / side).toInt()
        for (y in 0 until down) {
            for (x in 0 until across) {
                drawImage(
                    image,
                    dstOffset = IntOffset(x * side, y * side),
                    dstSize = IntSize(side, side),
                    filterQuality = FilterQuality.None,
                )
            }
        }
    }
}
