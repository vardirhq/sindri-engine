// The inspector: every control is a view of the recipe and an edit to its text.
//
// A control never holds state of its own. After each bake `sync` moves every
// control to the value the baker parsed, and moving a control rewrites the
// recipe's JSON and bakes again, so the text in the Recipe tab is always what
// Save recipe will write. A recipe the baker refuses can still be mended here,
// provided it is JSON.
const DEGREES = 180 / Math.PI;

/** Where the light is, as the two angles a person thinks in. */
function lightAngles([x, y, z]) {
  const length = Math.hypot(x, y, z) || 1;
  return {
    azimuth: Math.round(((Math.atan2(z, x) * DEGREES) + 360) % 360),
    elevation: Math.round(Math.asin(y / length) * DEGREES),
  };
}

function lightFrom(azimuth, elevation) {
  const flat = Math.cos(elevation / DEGREES);
  const round = (v) => Math.round(v * 1000) / 1000;
  return [
    round(flat * Math.cos(azimuth / DEGREES)),
    round(Math.sin(elevation / DEGREES)),
    round(flat * Math.sin(azimuth / DEGREES)),
  ];
}

/** The one model file a recipe bakes, or null when it is built from primitives or is a sheet of several. */
export function fileModelOf(recipe) {
  const [only] = recipe.variants;
  return recipe.variants.length === 1 && 'file' in only.model ? only.model : null;
}

/** What each control shows, read from a recipe the baker has parsed. */
const READ = {
  supersample: (r) => r.render.supersample,
  alpha_cutoff: (r) => r.render.alphaCutoff,
  padding: (r) => r.render.padding,
  palette_snap: (r) => r.render.paletteSnap,
  outline_enabled: (r) => r.render.outline.enabled,
  outline_color: (r) => r.render.outline.color,
  azimuth: (r) => lightAngles(r.render.shading.light).azimuth,
  elevation: (r) => lightAngles(r.render.shading.light).elevation,
  band0: (r) => r.render.shading.thresholds[0],
  band1: (r) => r.render.shading.thresholds[1],
  band2: (r) => r.render.shading.thresholds[2],
  footprint_w: (r) => r.footprint.width,
  footprint_h: (r) => r.footprint.height,
  directions: (r) => r.directions,
  facing: (r) => r.facing,
  auto_fit: (r) => fileModelOf(r)?.autoFit ?? false,
  fill: (r) => fileModelOf(r)?.fill ?? 1,
  scale: (r) => fileModelOf(r)?.scale ?? 1,
  rotation_y: (r) => Math.round(((fileModelOf(r)?.rotationY ?? 0) % 360 + 360) % 360),
  offset_x: (r) => fileModelOf(r)?.offset[0] ?? 0,
  offset_y: (r) => fileModelOf(r)?.offset[1] ?? 0,
  offset_z: (r) => fileModelOf(r)?.offset[2] ?? 0,
};

const two = (v) => v.toFixed(2);
const LABEL = {
  supersample: (v) => `${v}×`,
  alpha_cutoff: (v) => `${v}`,
  padding: (v) => `${v}px`,
  azimuth: (v) => `${v}°`,
  elevation: (v) => `${v}°`,
  band0: two,
  band1: two,
  band2: two,
  footprint_w: (v) => `${v}`,
  footprint_h: (v) => `${v}`,
  fill: (v) => `${Math.round(v * 100)}%`,
  scale: (v) => `${two(v)}×`,
  rotation_y: (v) => `${v}°`,
  offset_x: two,
  offset_y: two,
  offset_z: two,
};

const COLORS = /^#[0-9a-f]{6}$/i;

/**
 * `current()` returns `{ recipe, asset }` for the last good bake, and `rerun`
 * bakes the recipe text again.
 */
export function setupControls({ recipeBox, current, rerun }) {
  const inspector = document.getElementById('inspector');
  const inputs = Object.fromEntries(
    [...inspector.querySelectorAll('[data-setting]')].map((input) => [input.dataset.setting, input]),
  );
  const render = document.getElementById('controls');
  const model = document.getElementById('model-controls');
  const materials = document.getElementById('materials');
  let materialSignature = '';

  /** Rewrite the recipe text through `mutate`; false when the text is not JSON. */
  function edit(mutate) {
    let recipe;
    try {
      recipe = JSON.parse(recipeBox.value);
    } catch {
      return false;
    }
    mutate(recipe);
    recipeBox.value = `${JSON.stringify(recipe, null, 2)}\n`;
    rerun(0);
    return true;
  }

  function change(key, value) {
    const { recipe: parsed } = current();
    edit((recipe) => {
      const settings = (recipe.render ??= {});
      // A shading block needs both its keys, so start it from what the bake is using.
      const shading = () =>
        (settings.shading ??= {
          light: [...parsed.render.shading.light],
          thresholds: [...parsed.render.shading.thresholds],
        });
      const file = recipe.model?.kind === 'file' ? recipe.model : null;

      if (key === 'supersample' || key === 'alpha_cutoff' || key === 'padding' || key === 'palette_snap') {
        settings[key] = value;
      } else if (key === 'outline_enabled') {
        (settings.outline ??= {}).enabled = value;
      } else if (key === 'outline_color') {
        (settings.outline ??= {}).color = value;
      } else if (key === 'azimuth' || key === 'elevation') {
        const next = { ...lightAngles(shading().light), [key]: value };
        shading().light = lightFrom(next.azimuth, next.elevation);
      } else if (key.startsWith('band')) {
        // A band edge may not pass its neighbours: the baker requires them ascending.
        const index = Number(key.slice(4));
        const edges = shading().thresholds;
        edges[index] = Math.min(Math.max(value, edges[index - 1] ?? 0), edges[index + 1] ?? 1);
      } else if (key === 'footprint_w' || key === 'footprint_h') {
        recipe.footprint = { ...(recipe.footprint ?? { width: 1, height: 1 }) };
        recipe.footprint[key === 'footprint_w' ? 'width' : 'height'] = value;
      } else if (key === 'directions') {
        recipe.directions = Number(value);
      } else if (key === 'facing') {
        recipe.facing = value;
      } else if (file) {
        if (key.startsWith('offset_')) {
          const offset = [...(file.offset ?? fileModelOf(parsed)?.offset ?? [0, 0, 0])];
          offset['xyz'.indexOf(key.slice(-1))] = value;
          file.offset = offset;
        } else {
          file[key] = value;
        }
      }
    });
  }

  function setMaterial(name, color) {
    edit((recipe) => {
      const file = recipe.model;
      if (file?.kind !== 'file') return;
      file.materials ??= {};
      if (color === null) delete file.materials[name];
      else if (typeof file.materials[name] === 'object') file.materials[name].color = color;
      else file.materials[name] = color;
      if (Object.keys(file.materials).length === 0) delete file.materials;
    });
  }

  for (const [key, input] of Object.entries(inputs)) {
    input.addEventListener('input', () => {
      const value =
        input.type === 'checkbox' ? input.checked : input.type === 'color' || input.tagName === 'SELECT' ? input.value : Number(input.value);
      const out = inspector.querySelector(`[data-out="${key}"]`);
      if (out && LABEL[key]) out.textContent = LABEL[key](value);
      change(key, value);
    });
  }

  /** Rows of material colors, rebuilt only when the file's materials or overrides change. */
  function syncMaterials(recipe, asset) {
    const file = fileModelOf(recipe);
    const raw = file && asset.models[file.file];
    const rows = [];
    for (const part of raw?.parts ?? []) {
      if (!rows.some((row) => row.name === part.name)) rows.push({ name: part.name, original: part.color });
    }
    for (const row of rows) row.override = file.materials[row.name]?.color ?? null;
    const signature = JSON.stringify(rows);
    if (signature === materialSignature) return;
    materialSignature = signature;

    materials.replaceChildren(
      ...rows.map((row) => {
        const item = document.createElement('div');
        item.className = 'material';
        const name = document.createElement('span');
        name.textContent = row.name;
        const color = document.createElement('input');
        color.type = 'color';
        color.value = COLORS.test(row.override ?? '') ? row.override : row.original;
        color.setAttribute('aria-label', `${row.name} color`);
        color.addEventListener('input', () => setMaterial(row.name, color.value));
        const reset = document.createElement('button');
        reset.type = 'button';
        reset.textContent = 'Reset';
        reset.disabled = row.override === null;
        reset.setAttribute('aria-label', `Reset ${row.name} color`);
        reset.addEventListener('click', () => setMaterial(row.name, null));
        item.append(name, color, reset);
        return item;
      }),
    );
    if (file && rows.length === 0) materials.textContent = 'The model is not loaded.';
  }

  return {
    /** Enable the controls only while the recipe text is JSON they can edit. */
    setEditable(editable) {
      render.disabled = !editable;
      model.disabled = !editable;
    },

    /** Move every control to the recipe the baker last parsed. */
    sync(recipe, asset) {
      for (const [key, input] of Object.entries(inputs)) {
        if (input === document.activeElement) continue;
        const value = READ[key](recipe);
        if (input.type === 'checkbox') input.checked = value;
        else if (input.type === 'color') input.value = COLORS.test(value) ? value : '#000000';
        else input.value = value;
        const out = inspector.querySelector(`[data-out="${key}"]`);
        if (out && LABEL[key]) out.textContent = LABEL[key](Number(value));
      }
      const file = fileModelOf(recipe);
      document.getElementById('file-controls').hidden = !file;
      document.getElementById('primitive-note').hidden = !!file;
      document.getElementById('primitive-note').textContent =
        recipe.variants.length > 1
          ? 'This recipe is a sheet of several models. Edit them in the Recipe tab.'
          : 'This recipe is built from primitives. Edit its parts in the Recipe tab.';
      if (file) syncMaterials(recipe, asset);
    },
  };
}

/** The Render / Model / Recipe tabs, with arrow keys between them. */
export function setupTabs() {
  const tabs = [...document.querySelectorAll('#inspector [role=tab]')];
  const show = (selected) => {
    for (const tab of tabs) {
      const on = tab === selected;
      tab.setAttribute('aria-selected', String(on));
      tab.tabIndex = on ? 0 : -1;
      document.getElementById(tab.getAttribute('aria-controls')).hidden = !on;
    }
  };
  tabs.forEach((tab, index) => {
    tab.addEventListener('click', () => show(tab));
    tab.addEventListener('keydown', (event) => {
      const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
      if (!step) return;
      const next = tabs[(index + step + tabs.length) % tabs.length];
      show(next);
      next.focus();
    });
  });
  show(tabs[0]);
  return show;
}
