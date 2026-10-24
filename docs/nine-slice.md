# Nine-slice sprites

A sliced image keeps its border at the size the art drew it and stretches only
what is inside. One small picture of a frame becomes a panel of any size, a
board of any width, a bar whose rounded ends stay round.

## The border is the art's

The sheet beside an image says where its border runs, in pixels, by sprite
name. The empty name is the whole image:

```json
{
  "format_version": 1,
  "size": [24, 24],
  "grid": { "columns": 1, "rows": 1, "names": ["panel"] },
  "borders": { "": [6, 6, 6, 6] }
}
```

The four numbers are left, top, right and bottom. A border is pixels, so the
sheet must know the image's size: `size`, or the grid's own `size`. A sheet with
borders and no size is refused, as is a border naming a sprite the sheet does
not cut or one wider than its sprite. An image drawn whole with a sprite of its
own sheet uses that sprite's border, or the whole image's when the sprite has
none.

## Drawing sliced

`sindri.sprite` and `sindri.ui.image` both take:

| Field | Meaning | Default |
| --- | --- | --- |
| `draw` | `simple` stretches the whole picture; `sliced` keeps the border | `simple` |
| `border_scale` | How big one border pixel is drawn | sprite `1/16` world unit; UI `2/720` screen unit |

The image's size is its transform's scale, as it always was. Sliced, it is nine
quads: the corners keep their size, the top and bottom edges stretch across, the
sides stretch down, and the middle stretches both ways. They are drawn in the
image's own space, so a sliced image turns, flips and anchors as a whole one
does, and they share one draw call. An image smaller than its two borders
squeezes them in proportion rather than letting them cross.

The sprite default draws sixteen art pixels to a world unit, the size the
platformer's tiles are drawn at. The UI default draws one art pixel per screen
pixel on a 720-pixel-tall window and grows with the window as the rest of the UI
does.

A sliced UI image that fills (`fill`) shrinks as a whole, so a bar keeps its end
caps as it empties instead of cutting through one. A sliced image whose sheet
gives no border draws whole.

## In the editor

The Sprite sheet tab's inspector has a Nine-slice border section for the picked
cell, and the image shows the border as four dashed guides. Dragging a guide
moves that side a whole pixel at a time and stops it at the opposite guide.
Saving writes the border and the image size into the sheet. A border on a cell
since renamed is not written.

## Proof

The platformer's HUD frame and its signboard by the first gap are both
`textures/panel.png`, a 24-pixel panel with a six-pixel frame, drawn at very
different sizes. `games/lamplighter/tests/sliced_panels.rs` checks both slice.
