/**
 * Reducing a model's colors to a few.
 *
 * A textured model has thousands of colors and a sprite a few dozen pixels tall
 * has room for a handful, so the colors of a model's triangles are cut down to
 * `k` before any of them becomes a material. Median cut: repeatedly split the
 * group of colors with the most to lose, at its median, along its widest
 * channel, until there are `k` groups. It is deterministic -- ties break on
 * position, and nothing is random -- because a baked asset has to come out the
 * same every time it is baked.
 */

export interface Quantized {
  /** One RGB color per cluster, 0 to 255, the most common first. */
  palette: [number, number, number][];
  /** Which cluster each input color landed in. */
  assignment: Uint16Array;
}

/** `colors` is three bytes per color; the result has at most `k` clusters. */
export function quantizeColors(colors: ArrayLike<number>, k: number): Quantized {
  const count = Math.floor(colors.length / 3);
  if (count === 0 || !(k >= 1)) return { palette: [], assignment: new Uint16Array(count) };

  let boxes: number[][] = [Array.from({ length: count }, (_, index) => index)];
  const widest = (box: number[]): { axis: number; range: number } => {
    let best = { axis: 0, range: -1 };
    for (let axis = 0; axis < 3; axis++) {
      let low = 255;
      let high = 0;
      for (const index of box) {
        const value = colors[index * 3 + axis];
        if (value < low) low = value;
        if (value > high) high = value;
      }
      if (high - low > best.range) best = { axis, range: high - low };
    }
    return best;
  };

  while (boxes.length < Math.floor(k)) {
    let pick = -1;
    let pickScore = 0;
    let pickAxis = 0;
    boxes.forEach((box, at) => {
      if (box.length < 2) return;
      const { axis, range } = widest(box);
      const score = range * box.length;
      if (range > 0 && score > pickScore) {
        pick = at;
        pickScore = score;
        pickAxis = axis;
      }
    });
    if (pick < 0) break;

    const box = boxes[pick];
    const sorted = [...box].sort(
      (a, b) => colors[a * 3 + pickAxis] - colors[b * 3 + pickAxis] || a - b,
    );
    const middle = sorted.length >> 1;
    // Never split equal colors apart: move the cut to where the value changes.
    let cut = middle;
    const at = (i: number) => colors[sorted[i] * 3 + pickAxis];
    while (cut < sorted.length && cut > 0 && at(cut) === at(cut - 1)) cut++;
    if (cut >= sorted.length) {
      cut = middle;
      while (cut > 0 && at(cut) === at(cut - 1)) cut--;
    }
    if (cut <= 0 || cut >= sorted.length) break;
    boxes = [...boxes.slice(0, pick), sorted.slice(0, cut), sorted.slice(cut), ...boxes.slice(pick + 1)];
  }

  const summaries = boxes.map((box) => {
    const sum = [0, 0, 0];
    for (const index of box) for (let axis = 0; axis < 3; axis++) sum[axis] += colors[index * 3 + axis];
    const color = sum.map((total) => Math.round(total / box.length)) as [number, number, number];
    return { box, color };
  });
  summaries.sort(
    (a, b) => b.box.length - a.box.length || a.color[0] - b.color[0] || a.color[1] - b.color[1] || a.color[2] - b.color[2],
  );

  const assignment = new Uint16Array(count);
  summaries.forEach(({ box }, cluster) => {
    for (const index of box) assignment[index] = cluster;
  });
  return { palette: summaries.map(({ color }) => color), assignment };
}
