// A baked frame drawn where it will stand, not on a checkerboard.
//
// The floor is the tiles the recipe's footprint reserves, plus one tile of
// ground around them, projected through the bake's own camera. Because the
// anchor is the floor centre of footprint tile (0, 0) and sits at the centre of
// every frame, a frame lands on its floor by construction: if it floats or sinks
// here, it will in the game.
//
// One routine serves all three views. A tile's corners go through the same
// `projectToPixels` the bake used, so an isometric floor comes out as diamonds
// and a top-down one as squares. A side view looks along the ground, so its
// tiles have no area; that is drawn as a ground line with earth below it.
import { projectToPixels } from './baker/camera.js';
import { vec } from './baker/vec.js';

const FLOOR = { near: ['#9d8f77', '#8c7f6a'], far: ['#797060', '#6f6657'] };
const EARTH_DEPTH = 10;

function corners(camera, i, j) {
  const origin = vec(0, 0, 0);
  return [
    [-0.5, -0.5],
    [0.5, -0.5],
    [0.5, 0.5],
    [-0.5, 0.5],
  ].map(([dx, dz]) => projectToPixels(camera, vec(i + dx, 0, j + dz), origin));
}

function area(polygon) {
  let sum = 0;
  polygon.forEach((a, index) => {
    const b = polygon[(index + 1) % polygon.length];
    sum += a.x * b.y - b.x * a.y;
  });
  return Math.abs(sum) / 2;
}

/** The ground a frame stands on, as shapes in anchor-relative pixels. */
function floorFor(camera, footprint) {
  const tiles = [];
  for (let i = -1; i <= footprint.width; i++) {
    for (let j = -1; j <= footprint.height; j++) {
      const inside = i >= 0 && j >= 0 && i < footprint.width && j < footprint.height;
      tiles.push({ i, j, inside, polygon: corners(camera, i, j) });
    }
  }
  const flat = tiles.every((tile) => area(tile.polygon) < 1e-6);
  return { tiles, flat };
}

function spriteCanvas(image) {
  const canvas = document.createElement('canvas');
  canvas.width = image.width;
  canvas.height = image.height;
  canvas.getContext('2d').putImageData(new ImageData(image.data, image.width, image.height), 0, 0);
  return canvas;
}

/**
 * One frame of a bake, on its floor.
 *
 * `options.zoom` is a whole number of screen pixels per baked pixel and
 * `options.grid` says whether to draw the floor at all.
 */
export function framePreview(result, index, { zoom, grid }) {
  const frame = result.frames[index];
  const [anchorX, anchorY] = result.report.anchor;
  const { image } = frame;
  const floor = grid ? floorFor(result.camera, frame.footprint) : null;

  // Bounds in anchor-relative pixels: the sprite and the tiles it stands on.
  // The ring of ground around them is drawn too, but runs off the edges, so a
  // card is as big as its sprite needs rather than as big as its floor.
  let minX = -anchorX;
  let minY = -anchorY;
  let maxX = image.width - anchorX;
  let maxY = image.height - anchorY;
  if (floor) {
    for (const tile of floor.tiles.filter((t) => t.inside)) {
      for (const point of tile.polygon) {
        minX = Math.min(minX, point.x);
        maxX = Math.max(maxX, point.x);
        minY = Math.min(minY, point.y);
        maxY = Math.max(maxY, point.y);
      }
    }
    if (floor.flat) maxY = Math.max(maxY, EARTH_DEPTH);
  }
  minX = Math.floor(minX) - 4;
  minY = Math.floor(minY) - 4;
  maxX = Math.ceil(maxX) + 4;
  maxY = Math.ceil(maxY) + 4;

  const canvas = document.createElement('canvas');
  canvas.width = (maxX - minX) * zoom;
  canvas.height = (maxY - minY) * zoom;
  const ctx = canvas.getContext('2d');
  ctx.imageSmoothingEnabled = false;
  ctx.scale(zoom, zoom);
  ctx.translate(-minX, -minY);

  if (floor && floor.flat) {
    const left = minX;
    const right = maxX;
    ctx.fillStyle = FLOOR.far[1];
    ctx.fillRect(left, 0, right - left, EARTH_DEPTH);
    ctx.fillStyle = FLOOR.near[0];
    ctx.fillRect(left, 0, right - left, 1);
    // Tick marks where tiles meet, so a footprint's width can be read.
    ctx.fillStyle = 'rgba(0,0,0,0.3)';
    for (const tile of floor.tiles.filter((t) => t.j === 0)) {
      ctx.fillRect(tile.polygon[0].x, 0, 0.5, EARTH_DEPTH);
    }
  } else if (floor) {
    ctx.lineWidth = 1 / zoom;
    ctx.strokeStyle = 'rgba(0,0,0,0.22)';
    for (const tile of floor.tiles) {
      ctx.beginPath();
      tile.polygon.forEach((point, at) => (at ? ctx.lineTo(point.x, point.y) : ctx.moveTo(point.x, point.y)));
      ctx.closePath();
      ctx.fillStyle = (tile.inside ? FLOOR.near : FLOOR.far)[(tile.i + tile.j) & 1];
      ctx.fill();
      ctx.stroke();
    }
  }

  ctx.drawImage(spriteCanvas(image), Math.round(-anchorX), Math.round(-anchorY));
  return canvas;
}
