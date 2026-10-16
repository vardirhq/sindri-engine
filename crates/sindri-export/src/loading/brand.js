
  // ---- The handoff from Sindri to a project's brand, and its entrances. ----
  //
  // When a project has a brand, the Sindri concept does not fade away: the
  // ENGINE tag flips to PRESENTS, the mark and the wordmark fall apart, and
  // the brand's background rises behind them. Then the brand makes its
  // entrance, in one of the Sindri styles: struck on the anvil, drafted as a
  // blueprint, stacked out of its own pixels, switched on like a CRT, or
  // unlocked like an achievement. Each is a function of `e`, 0 to 1 across the
  // entrance, and holds its last frame for as long as the brand stays.

  const HANDOFF_MS = 1500;
  const ENTRANCE_MS = 2000;
  // What a brand holds still for, at least, before the game may replace it,
  // so an entrance never ends on the frame the screen fades.
  const HOLD_MS = 500;

  const ENTRANCES = ["forge", "blueprint", "pixel", "arcade", "achievement"];

  // The brand as the page's still stage describes it. The image element is
  // already loading, so the entrances use what it fetched.
  const readBrand = (stage) => {
    const image = stage.querySelector("img");
    const background = getComputedStyle(screen).getPropertyValue("--brand-background").trim();
    const brand = {
      image: image ? image.getAttribute("src") : null,
      alt: image ? image.getAttribute("alt") : "",
      title: stage.querySelector(".brand-title")?.textContent ?? "",
      caption: stage.querySelector(".brand-caption")?.textContent ?? "",
      background: background || INK,
      entrance: screen.dataset.brandEntrance || "random",
      size: null,
      pixels: null,
    };
    if (brand.entrance === "random" || !(ENTRANCES.includes(brand.entrance) || brand.entrance === "fade")) {
      let last = null;
      try { last = localStorage.getItem("sindri-loading-entrance"); } catch {}
      const choices = ENTRANCES.filter((name) => name !== last);
      brand.entrance = choices[Math.floor(Math.random() * choices.length)];
      try { localStorage.setItem("sindri-loading-entrance", brand.entrance); } catch {}
    }
    if (brand.image) {
      const source = new Image();
      source.onload = () => {
        brand.size = [source.naturalWidth || 300, source.naturalHeight || 150];
        if (brand.entrance === "pixel") brand.pixels = pixelsOf(source, brand.size);
      };
      source.src = brand.image;
    }
    return brand;
  };

  // The image as blocks, and at the coarse resolutions the pixel entrance
  // sharpens through. `null` if the browser will not let the page read it.
  const pixelsOf = (source, [nw, nh]) => {
    try {
      const sample = (cols) => {
        const rows = Math.max(1, Math.round((cols * nh) / nw));
        const canvas = document.createElement("canvas");
        canvas.width = cols;
        canvas.height = rows;
        const context = canvas.getContext("2d", { willReadFrequently: true });
        context.drawImage(source, 0, 0, cols, rows);
        return { canvas, context, cols, rows };
      };
      const blocks = sample(nw >= nh ? 22 : Math.max(4, Math.round((22 * nw) / nh)));
      const data = blocks.context.getImageData(0, 0, blocks.cols, blocks.rows).data;
      const cells = [];
      for (let y = 0; y < blocks.rows; y++) {
        for (let x = 0; x < blocks.cols; x++) {
          const at = (y * blocks.cols + x) * 4;
          if (data[at + 3] < 40) continue;
          cells.push({ x, y, color: `rgba(${data[at]},${data[at + 1]},${data[at + 2]},${(data[at + 3] / 255).toFixed(3)})` });
        }
      }
      const steps = [44, 88].map((cols) => sample(cols).canvas.toDataURL());
      return { cols: blocks.cols, rows: blocks.rows, cells, steps };
    } catch {
      return null;
    }
  };

  // Where the image sits: as large as the still stage would show it, never
  // larger than it is.
  const imageBox = (brand, S, w, h) => {
    const [nw, nh] = brand.size || [0, 0];
    if (!nw || !nh) return null;
    const scale = Math.min((Math.min(w * 0.74, h * 0.62, 760)) / nw, (h * 0.4) / nh, 1);
    return [nw * scale, nh * scale];
  };

  const titleStyle = { font: "700 clamp(1.4rem, 5vmin, 3rem)/1.1 system-ui, sans-serif", color: CREAM, textAlign: "center" };
  const captionStyle = { font: "400 clamp(.85rem, 2vmin, 1.1rem)/1.4 system-ui, sans-serif", color: CREAM, textAlign: "center" };

  // The brand's column, centered on its background. `parts` styles or
  // replaces each piece, and `over` draws on top of the whole screen.
  function BrandColumn({ brand, S, w, h, parts = {}, under, over, column }) {
    const box = imageBox(brand, S, w, h);
    // A part an entrance names replaces the plain one, and naming it `null`
    // leaves it out.
    const image = "image" in parts ? parts.image : brand.image && box ? node("img", {
      src: brand.image,
      alt: brand.alt,
      style: { display: "block", width: box[0], height: box[1], ...parts.imageStyle },
    }) : null;
    const title = "title" in parts ? parts.title : brand.title ? node("div", { style: { ...titleStyle, ...parts.titleStyle } }, brand.title) : null;
    const caption = "caption" in parts ? parts.caption : brand.caption ? node("div", { style: { ...captionStyle, opacity: 0.75, ...parts.captionStyle } }, brand.caption) : null;
    return node("div", { style: { position: "absolute", inset: 0, background: brand.background, overflow: "hidden" } },
      under,
      node("div", {
        style: {
          position: "absolute", inset: 0, display: "flex", flexDirection: "column",
          alignItems: "center", justifyContent: "center", gap: 24 * S, ...column,
        },
      }, image, title, caption),
      over);
  }

  // ---- The handoff ----

  // The mark as the pieces it falls apart into: each edge of both hexes, and
  // the core, so that at rest they draw exactly the mark.
  const edges = (V) => V.map((a, i) => [a, V[(i + 1) % V.length]]);
  const PIECES = [
    ...edges(OUT_V).map((pair, i) => ({ pair, width: 1.6, color: CREAM, k: i })),
    ...edges(IN_V).map((pair, i) => ({ pair, width: 1.2, color: CREAM, k: 6 + i })),
  ];

  function HandoffMark({ k: h, M }) {
    const fall = (k) => {
      const u = since01(h, 0.3 + 0.03 * (k % 6) + 0.05 * rnd(k, 7), 0.5);
      return u < 0 ? (h < 0.3 ? 0 : 1) : u;
    };
    const lines = PIECES.map(({ pair: [a, b], width, color, k }) => {
      const u = fall(k);
      const dx = (rnd(k, 1) - 0.5) * 20 * u;
      const dy = 60 * u * u;
      const cx = (a[0] + b[0]) / 2, cy = (a[1] + b[1]) / 2;
      const turn = (rnd(k, 2) - 0.5) * 400 * u;
      return node("line", {
        key: k, x1: a[0], y1: a[1], x2: b[0], y2: b[1], stroke: color, strokeWidth: width,
        strokeLinecap: "square",
        transform: `translate(${dx} ${dy}) rotate(${turn} ${cx} ${cy})`,
        opacity: 1 - u,
      });
    });
    const u = fall(12);
    return node("svg", { width: M, height: M, viewBox: "0 0 32 32", fill: "none", style: { display: "block", flex: "none", overflow: "visible" } },
      lines,
      node("g", { transform: `translate(0 ${70 * u * u}) translate(16 16) scale(${1 - 0.4 * u}) translate(-16 -16)`, opacity: 1 - u },
        node("polygon", { points: CORE, fill: GOLD })));
  }

  // ENGINE flips to PRESENTS, the mark and letters fall, the brand's
  // background rises, and PRESENTS is the last thing to go.
  function Handoff({ k: h, S, w, horizontal, brand, height }) {
    const M = (horizontal ? 420 : 470) * S;
    const font = (horizontal ? 150 : 124) * S;
    const flip = clamp(h / 0.24);
    const letterFall = (i) => {
      const u = clamp((h - 0.36 - 0.035 * i) / 0.42);
      return {
        transform: `translate(${(rnd(i, 9) - 0.5) * 60 * S * u}px, ${height * 0.7 * u * u}px) rotate(${(rnd(i, 4) - 0.5) * 70 * u}deg)`,
        opacity: 1 - u * u,
      };
    };
    const tag = {
      text: flip < 0.5 ? "ENGINE" : "PRESENTS",
      ruleScale: 1 - clamp((h - 0.4) / 0.3),
      chip: {
        transform: `scaleY(${Math.abs(Math.cos(Math.PI * flip))})`,
        background: flip < 0.5 ? CREAM : GOLD,
        opacity: 1 - clamp((h - 0.82) / 0.16),
      },
    };
    return node("div", { style: { position: "absolute", inset: 0 } },
      node("div", { style: { position: "absolute", inset: 0, background: brand.background, opacity: clamp((h - 0.5) / 0.4) } }),
      node(Layout, {
        S, horizontal, showProgress: true, bare: true,
        mark: node(HandoffMark, { k: h, M }),
        word: node(Wordmark, { S, font, iColor: GOLD, showNext: true, tag, letterStyle: letterFall }),
        progress: node(Progress, { p: h, S, w, color: GOLD, label: "Loading.." }),
      }));
  }

  // ---- The entrances ----

  // A plain rise, for a brand that asks for nothing more.
  function FadeIn({ e, S, w, h, brand }) {
    const rise = (start) => {
      const u = easeOut((e - start) / 0.45);
      return { opacity: u, transform: `translateY(${(1 - u) * 24 * S}px)` };
    };
    return node(BrandColumn, { brand, S, w, h, parts: { imageStyle: rise(0), titleStyle: rise(0.12), captionStyle: { ...rise(0.24), opacity: 0.75 * easeOut((e - 0.24) / 0.45) } } });
  }

  // Dropped on the anvil: it falls in glowing, strikes, throws sparks off its
  // base and cools while the title heats and cools beside it.
  const FORGE_STRIKE = 0.22;
  function ForgeIn({ e, S, w, h, brand }) {
    const box = imageBox(brand, S, w, h) || [0, 0];
    const fallen = clamp(e / FORGE_STRIKE);
    const after = e - FORGE_STRIKE;
    const imp = after >= 0 ? Math.exp(-after * 14) : 0;
    const heat = after < 0 ? 0.7 : 0.12 + 0.88 * Math.pow(1 - clamp(after / (1 - FORGE_STRIKE)), 2.2);
    const hc = heatRGB(0.45 + 0.55 * heat);
    const drop = after < 0 ? -(1 - fallen * fallen) * h * 0.6 : 0;
    const imageStyle = {
      transform: `translateY(${drop}px) scale(${1 + 0.05 * imp}, ${1 - 0.1 * imp})`,
      transformOrigin: "50% 100%",
      filter: `drop-shadow(0 0 ${36 * S * heat}px ${rgb(hc, 0.8 * heat)}) brightness(${1 + 0.6 * heat}) sepia(${0.7 * heat}) saturate(${1 + 1.5 * heat})`,
      opacity: after < 0 ? 0.3 + 0.7 * fallen : 1,
    };
    const tc = heatRGB(0.55 + 0.45 * heat);
    const cooled = clamp(after / 0.6);
    const titleStyle = {
      color: rgb([0, 1, 2].map((j) => Math.round(tc[j] * (1 - cooled) + [230, 225, 212][j] * cooled))),
      textShadow: `0 0 ${30 * S * heat}px ${rgb(hc, 0.7 * heat)}`,
      opacity: after < 0 ? 0 : easeOut(after / 0.12),
      transform: `translateY(${-imp * 12 * S}px)`,
    };
    // Sparks off the base of the image, in screen pixels.
    const sparks = [];
    const gap = 24 * S;
    const block = box[1] + (brand.title ? gap + 48 * S : 0) + (brand.caption ? gap + 20 * S : 0);
    const baseY = h / 2 - block / 2 + box[1];
    if (after >= 0) {
      for (let i = 0; i < 44; i++) {
        const life = 0.22 + 0.3 * rnd(i, 3);
        const t = after / life;
        if (t >= 1) continue;
        const side = rnd(i, 6) < 0.5 ? -1 : 1;
        const x0 = w / 2 + side * box[0] * (0.1 + 0.4 * rnd(i, 8));
        const a = -Math.PI / 2 + side * (0.3 + 1.1 * rnd(i, 1));
        const v = (180 + 420 * rnd(i, 2)) * S;
        const at = (tt) => [x0 + Math.cos(a) * v * tt * life, baseY + Math.sin(a) * v * tt * life + 900 * S * tt * tt * life * life];
        const [x1, y1] = at(t);
        const [x2, y2] = at(Math.max(0, t - 0.12));
        sparks.push(node("line", { key: i, x1: x2, y1: y2, x2: x1, y2: y1, stroke: rgb(heatRGB(1 - t * 0.5)), strokeWidth: (3 + 3 * rnd(i, 5)) * S, strokeLinecap: "round", opacity: 1 - t * t }));
      }
    }
    const ring = after >= 0 && after < 0.4 ? easeOut(after / 0.4) : -1;
    const over = node("svg", { width: w, height: h, viewBox: `0 0 ${w} ${h}`, style: { position: "absolute", inset: 0, pointerEvents: "none", overflow: "visible" } },
      ring >= 0 ? node("ellipse", { cx: w / 2, cy: baseY, rx: box[0] * (0.5 + 0.5 * ring), ry: 14 * S * (0.5 + ring), fill: "none", stroke: rgb(hc), strokeWidth: 3 * S * (1 - ring), opacity: 0.7 * (1 - ring) }) : null,
      sparks);
    const under = node("div", { style: { position: "absolute", inset: 0, background: `radial-gradient(40% 40% at 50% 50%, ${rgb(hc, 0.18 * heat)}, transparent)` } });
    return node(BrandColumn, { brand, S, w, h, under, over, parts: { imageStyle, titleStyle, captionStyle: { opacity: 0.75 * easeOut((e - 0.55) / 0.3) } } });
  }

  // Drafted: a grid, a frame drawn edge by edge with its dimensions, the image
  // as faint line-work, then inked from left to right; the title is typed.
  function BlueprintIn({ e, S, w, h, brand }) {
    const box = imageBox(brand, S, w, h);
    const guides = 1 - clamp((e - 0.82) / 0.18);
    const frame = easeOut(e / 0.38);
    const ink = clamp((e - 0.42) / 0.32);
    const draft = clamp((e - 0.18) / 0.2);
    const cell = 60 * S;
    const under = node("div", {
      style: {
        position: "absolute", inset: 0, opacity: 0.45 * guides * easeOut(e / 0.2),
        backgroundImage: `linear-gradient(${LINE} 1px, transparent 1px), linear-gradient(90deg, ${LINE} 1px, transparent 1px)`,
        backgroundSize: `${cell}px ${cell}px`, backgroundPosition: "center center",
      },
    });
    let image = null;
    let over = null;
    if (box) {
      const [bw, bh] = box;
      const pad = 14 * S;
      const perimeter = 2 * (bw + bh + 4 * pad);
      image = node("div", { style: { position: "relative", width: bw, height: bh } },
        node("img", { src: brand.image, alt: brand.alt, style: { position: "absolute", inset: 0, width: bw, height: bh, filter: "grayscale(1) brightness(1.7) contrast(.55)", opacity: 0.35 * draft * (1 - ink) } }),
        node("img", { src: brand.image, alt: "", style: { position: "absolute", inset: 0, width: bw, height: bh, clipPath: `inset(0 ${100 - ink * 100}% 0 0)` } }),
        ink > 0 && ink < 1 ? node("div", { style: { position: "absolute", top: -pad, bottom: -pad, left: bw * ink, width: Math.max(1, 2 * S), background: GOLD, boxShadow: `0 0 ${12 * S}px ${GOLD}` } }) : null,
        node("svg", { width: bw + 2 * pad, height: bh + 2 * pad, style: { position: "absolute", left: -pad, top: -pad, overflow: "visible", opacity: guides } },
          node("rect", { x: 0.5, y: 0.5, width: bw + 2 * pad - 1, height: bh + 2 * pad - 1, fill: "none", stroke: MUTED, strokeWidth: Math.max(1, 1.5 * S), strokeDasharray: perimeter, strokeDashoffset: perimeter * (1 - frame) }),
          node("line", { x1: 0, y1: bh + 2 * pad + 18 * S, x2: (bw + 2 * pad) * frame, y2: bh + 2 * pad + 18 * S, stroke: MUTED, strokeWidth: Math.max(1, S) }),
          node("text", {
            x: (bw + 2 * pad) / 2, y: bh + 2 * pad + 44 * S, textAnchor: "middle", fill: MUTED, opacity: clamp((e - 0.3) / 0.12),
            style: { fontFamily: "'JetBrains Mono', monospace", fontSize: 16 * S, letterSpacing: "0.2em" },
          }, `${brand.size[0]} × ${brand.size[1]}`)));
      over = node("div", { style: { position: "absolute", inset: 0, pointerEvents: "none", opacity: 0.5 * guides * (1 - frame * 0.6) } },
        node("div", { style: { position: "absolute", left: 0, right: 0, top: "50%", height: Math.max(1, S), background: MUTED, transform: `scaleX(${frame})` } }),
        node("div", { style: { position: "absolute", top: 0, bottom: 0, left: "50%", width: Math.max(1, S), background: MUTED, transform: `scaleY(${frame})` } }));
    }
    const typedTo = Math.floor(brand.title.length * clamp((e - 0.55) / 0.28));
    const caret = e > 0.5 && e < 0.95 && frac(e * 6) < 0.5;
    const title = node("div", { style: { ...titleStyle, opacity: e > 0.52 ? 1 : 0 } },
      typed(brand.title, typedTo),
      node("span", { style: { display: "inline-block", width: "0.5em", height: "0.9em", marginLeft: "0.08em", verticalAlign: "-0.1em", background: GOLD, opacity: caret ? 1 : 0 } }));
    return node(BrandColumn, { brand, S, w, h, under, over, parts: { image, title: brand.title ? title : null, captionStyle: { opacity: 0.75 * clamp((e - 0.84) / 0.12) } } });
  }

  // Stacked from its own pixels, bottom row first, then sharpened through
  // coarse resolutions to the image itself; the title steps in letter by
  // letter.
  function PixelIn({ e, S, w, h, brand }) {
    const box = imageBox(brand, S, w, h);
    const px = brand.pixels;
    if (!box || !px) return node(FadeIn, { e, S, w, h, brand });
    const [bw, bh] = box;
    const cw = bw / px.cols, ch = bh / px.rows;
    let image;
    if (e < 0.62) {
      const rects = px.cells.map((c, i) => {
        const land = 0.04 + 0.42 * ((px.rows - 1 - c.y) / Math.max(1, px.rows - 1)) + 0.08 * rnd(i, 3);
        const u = clamp((e - (land - 0.12)) / 0.12);
        if (u <= 0) return null;
        const y = c.y * ch - (1 - u * u) * h * 0.5;
        return node("rect", { key: i, x: c.x * cw, y, width: cw + 0.5, height: ch + 0.5, fill: c.color });
      });
      image = node("svg", { width: bw, height: bh, viewBox: `0 0 ${bw} ${bh}`, style: { display: "block", overflow: "visible" } }, rects);
    } else {
      const step = e < 0.72 ? px.steps[0] : e < 0.82 ? px.steps[1] : brand.image;
      image = node("img", { src: step, alt: brand.alt, style: { display: "block", width: bw, height: bh, imageRendering: "pixelated" } });
    }
    const shown = Math.ceil(brand.title.length * clamp((e - 0.66) / 0.24));
    const title = node("div", { style: titleStyle }, typed(brand.title, shown));
    return node(BrandColumn, { brand, S, w, h, parts: { image, title: brand.title ? title : null, captionStyle: { opacity: e > 0.9 ? 0.75 : 0 } } });
  }

  // Switched on: a dot, a line, the picture opening out of it overbright and
  // split into colour, a glitch, then a clean screen.
  function ArcadeIn({ e, S, w, h, brand }) {
    let sy = 1, bright = 1, show = true, beamW = 0, beamH = 0, beamOp = 0;
    if (e < 0.14) {
      show = false;
      if (e < 0.05) { const t = easeOut(e / 0.05); beamW = beamH = 18 * S * t; beamOp = t; }
      else { const t = easeOut((e - 0.05) / 0.09); beamW = 18 * S + (w * 0.86 - 18 * S) * t; beamH = 18 * S - 12 * S * t; beamOp = 1; }
    } else if (e < 0.26) {
      const t = easeOut((e - 0.14) / 0.12); sy = 0.01 + 0.99 * t; bright = 3 - 2 * t; beamOp = 1 - t; beamW = w * 0.86; beamH = 6 * S;
    }
    const glitch = e > 0.6 && e < 0.64 ? 1 : 0;
    const step = Math.floor(e * 120);
    const split = S * (2 + 18 * glitch + 16 * (1 - clamp((e - 0.14) / 0.4)));
    const settle = 1 - clamp((e - 0.8) / 0.2);
    const column = {
      opacity: show ? 1 : 0,
      transform: `translateX(${glitch ? (rnd(step, 1) - 0.5) * 40 * S : 0}px) scaleY(${sy})`,
      filter: `brightness(${bright}) drop-shadow(${-split * settle}px 0 0 rgba(255,40,90,0.7)) drop-shadow(${split * settle}px 0 0 rgba(40,220,255,0.6))`,
    };
    const scan = Math.max(1, 2 * S), gapPx = Math.max(2, 5 * S);
    const over = node("div", { style: { position: "absolute", inset: 0, pointerEvents: "none" } },
      node("div", { style: { position: "absolute", left: "50%", top: "50%", width: beamW, height: beamH, marginLeft: -beamW / 2, marginTop: -beamH / 2, background: "#fff7e0", opacity: beamOp, boxShadow: `0 0 ${30 * S}px ${8 * S}px rgba(255,240,200,0.5)`, borderRadius: beamH } }),
      node("div", { style: { position: "absolute", inset: 0, opacity: 0.25 + 0.75 * settle, backgroundImage: `repeating-linear-gradient(0deg, rgba(0,0,0,0.32) 0px, rgba(0,0,0,0.32) ${scan}px, rgba(0,0,0,0) ${scan}px, rgba(0,0,0,0) ${gapPx}px)` } }),
      node("div", { style: { position: "absolute", inset: 0, opacity: settle, background: "radial-gradient(120% 110% at 50% 50%, rgba(0,0,0,0) 55%, rgba(0,0,0,0.6) 100%)" } }));
    return node(BrandColumn, { brand, S, w, h, column, over });
  }

  // Unlocked: a toast drops in with the brand as its icon, a gold sheen
  // crosses it, and it settles in the middle of the screen.
  function AchievementIn({ e, S, w, h, brand }) {
    // The tile takes the image's shape, up to twice as wide as it is tall,
    // so a wide logo is not shrunk into a square.
    const box = imageBox(brand, S, w, h);
    const iconH = Math.min(190 * S, w * 0.24);
    const icon = box ? Math.min(iconH * Math.max(1, Math.min(2, box[0] / box[1])), w * 0.36) : iconH;
    const fit = box ? Math.min(icon * 0.86 / box[0], iconH * 0.86 / box[1]) : 0;
    const drop = Easing.easeInOutCubic(clamp(e / 0.24));
    const pop = since01(e, 0.28, 0.2);
    const sheen = since01(e, 0.36, 0.22);
    const pulse = since01(e, 0.3, 0.4);
    const panelW = Math.min(w * 0.9, 760 * S + icon);
    const panel = node("div", {
      style: {
        position: "relative", display: "flex", alignItems: "center", gap: 28 * S, padding: 24 * S, width: panelW,
        background: PANEL, overflow: "hidden", borderRadius: 6 * S,
        boxShadow: `inset 0 0 0 ${Math.max(1, 2 * S)}px ${pulse >= 0 ? rgb([240, 192, 80], 1 - pulse * 0.7) : LINE}, 0 ${20 * S}px ${60 * S}px rgba(0,0,0,0.5)`,
        transform: `translateY(${(1 - drop) * -(h / 2 + 200 * S)}px)`,
      },
    },
      node("div", {
        style: {
          flex: "none", width: icon, height: iconH, background: TILE, display: "flex", alignItems: "center", justifyContent: "center",
          transform: `scale(${pop >= 0 ? 1 + 0.18 * Math.sin(Math.PI * pop) : 1})`,
          boxShadow: pop >= 0 ? `0 0 ${40 * S * (1 - pop)}px ${GOLD}` : "none",
        },
      }, box ? node("img", { src: brand.image, alt: brand.alt, style: { display: "block", width: box[0] * fit, height: box[1] * fit } }) : null),
      node("div", { style: { display: "flex", flexDirection: "column", gap: 10 * S, minWidth: 0 } },
        node("div", { style: { fontFamily: MONO, fontSize: 18 * S, letterSpacing: "0.28em", color: GOLD, textTransform: "uppercase" } }, "Achievement unlocked"),
        brand.title ? node("div", { style: { ...titleStyle, textAlign: "left" } }, brand.title) : null,
        brand.caption ? node("div", { style: { ...captionStyle, textAlign: "left", opacity: 0.75 * clamp((e - 0.45) / 0.2) } }, brand.caption) : null),
      sheen >= 0 ? node("div", { style: { position: "absolute", top: 0, bottom: 0, width: "30%", left: `${-30 + 140 * sheen}%`, background: "linear-gradient(100deg, transparent, rgba(240,192,80,0.28), transparent)", pointerEvents: "none" } }) : null);
    return node(BrandColumn, { brand, S, w, h, parts: { image: panel, title: null, caption: null } });
  }

  const ENTRANCE = { fade: FadeIn, forge: ForgeIn, blueprint: BlueprintIn, pixel: PixelIn, arcade: ArcadeIn, achievement: AchievementIn };

  // The brand's side of the screen, `t` milliseconds after the page turned to
  // it: the concept's last frame giving way, the handoff, then the entrance,
  // with the bar under it all the way.
  function BrandScreen({ t, p, S, w, h, horizontal, brand, Concept }) {
    const props = { S, w, h, horizontal, brand, height: h };
    let scene;
    if (t < HANDOFF_MS) {
      const hh = t / HANDOFF_MS;
      const fadeConcept = 1 - clamp(hh / 0.12);
      scene = node("div", { style: { position: "absolute", inset: 0 } },
        node(Handoff, { ...props, k: hh }),
        fadeConcept > 0 ? node("div", { style: { position: "absolute", inset: 0, opacity: fadeConcept } },
          node(Concept, { p: 0, S, w, h, horizontal, showNext: true, showProgress: true, label: "Loading.." })) : null);
    } else {
      const e = clamp((t - HANDOFF_MS) / ENTRANCE_MS);
      scene = node("div", { style: { position: "absolute", inset: 0 } },
        node(ENTRANCE[brand.entrance] || FadeIn, { ...props, e }),
        node("div", { style: { position: "absolute", left: 0, right: 0, bottom: 110 * S, display: "flex", justifyContent: "center" } },
          node(Progress, { p, S, w, color: GOLD, label: "Loading.." })));
    }
    return scene;
  }
