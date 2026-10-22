# World text and rich text

Two additions to text, which share one renderer with the HUD's labels.

- `sindri.text` places words in the world: a damage number over an enemy, a
  name over a character, a sign over a door. It moves with its entity and the
  camera, and is drawn with the sprites of its layer.
- Rich text lets a run of the words carry its own color, bold or italic, in
  `sindri.ui.text` and `sindri.text` alike.

## `sindri.text`

Every field is `sindri.ui.text`'s and means the same thing, with two
differences that come from where the words live:

- **Sizes are world units.** `font_size`, `line_height`, `bounds`,
  `letter_spacing`, the outline width and the shadow offset are all measured
  in the world, so a number half a unit tall stays half as tall as a one-unit
  enemy whatever the camera's zoom. The overlay's two-unit ceiling on a font
  size does not apply.
- **The anchor names a point of the words, not of the screen.** `center` puts
  the middle of the words on the entity, `bottom` puts their bottom edge there,
  and so on. There is no viewport edge to anchor to.

The words sit at the entity's world position, parents included. Its scale does
not reach them: a label keeps its authored size, and a script that wants a
number to pop writes `this.world_text.font_size`.

Its rotation reaches them only with `follow_rotation: true`. Off, the default,
a label stays level however the entity under it spins, which is what a damage
number or a name plate wants. On, the words turn with the entity's world
rotation about their anchor point, shadow and outline with them, for words
painted on a crate or a sign hung at an angle.

```json
"sindri.text": {
  "text": "{.0}",
  "font": "fonts/ChakraPetch-Bold.ttf",
  "font_size": 0.34,
  "line_height": 0.4,
  "color": [0.86, 0.97, 1.0, 1.0],
  "anchor": "center",
  "layer": 20,
  "values": [10.0],
  "outline": { "width": 0.03, "color": [0.02, 0.05, 0.1, 0.9] },
  "rich": true
}
```

### Ordering

World text is drawn in the transparent 2D stage through the world camera, one
batch per layer. Within a layer it is drawn after the sprites and world shapes,
so a number is over the thing it labels; a sprite in a higher layer covers it.
It does not test depth, the same as the HUD's text.

### Scripts

The words are written the way a HUD label's are, with `Ui.set_text` and
`Ui.set_number`, which accept either component. The rest is on `this.world_text`:

| Path | Type |
| --- | --- |
| `this.world_text.color`, `this.world_text.color.{r,g,b,a}` | `Color`, `number` |
| `this.world_text.font_size` | `number` |
| `this.world_text.visible` | `number` |
| `this.world_text.layer` | `number` |

## Rich text

With `rich: true`, four tags in the words are read as spans instead of drawn:

| Tag | Meaning |
| --- | --- |
| `[b]…[/b]` | bold |
| `[i]…[/i]` | italic |
| `[color=#rrggbb]…[/color]` | the run's own color; `#rrggbbaa` gives it an alpha |
| `[[` | a literal `[` |

Tags nest, and `[/color]` returns to the color outside it. A tag that is not one
of these, or a closer with nothing open, is drawn as written, for the reason a
template's stray brace is: a typo should be on the screen where it is seen.

A span's color is multiplied by the text's own alpha, so fading `color.a` fades
every span with it. Bold and italic ask the face for its own weight and slant,
as the component-wide `bold` and `italic` do. A template's `{}` slots are
filled first, so a span can wrap a number: `[b][color=#ffd34d]{.0}![/color][/b]`.

Off by default, so a label that happens to hold a bracket draws as it always
did. Measurement, wrapping, auto-size and reveals all count the words without
their tags.

## Proof

Orbital's shots show their damage. Every landed shot spawns
`prefabs/damage-number.prefab`, world text that pops, drifts up and fades over
three quarters of a second; a critical hit is a bold gold rich span.
`games/orbital-baked/tests/damage_numbers.rs` plays until one lands and watches
it clear.

## Limits

- No per-glyph animation.
- No inline images or size changes inside a string; spans change color, weight
  and slant only.
- One face per string. Font fallback is still missing.
