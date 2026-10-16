// The loading screen's animations, one chosen at random each time the page
// opens. A classic script inlined in the page rather than a file it fetches, so
// it runs during the parse that shows the screen: there is nothing to wait for
// before the first frame, which is the whole of the wait the screen covers.
//
// Each concept is a pure, cyclic function of `p`, the place in a four-second
// loop, so the last frame meets the first. They were designed as JSX and are
// kept here as the calls JSX compiles to, against `node` below: a tree of
// plain objects, rendered by patching the elements of the frame before, so the
// page needs no framework for twelve kilobytes of motion.
//
// With `prefers-reduced-motion` none of this runs, and the still forge mark in
// the page's own markup stays as the loading screen.
(() => {
  const screen = document.querySelector("#sindri-loading");
  if (!screen || matchMedia("(prefers-reduced-motion: reduce)").matches) return;

  const LOOP_MS = 4000;
  const SVG = "http://www.w3.org/2000/svg";
  // React's rule for a bare number in a style: pixels, except for these.
  const UNITLESS = new Set(["opacity", "flex", "flexGrow", "flexShrink", "zIndex", "fontWeight", "lineHeight", "order"]);
  // SVG attributes whose names are camelCase in the markup too.
  const CAMEL = new Set(["viewBox", "patternUnits", "patternContentUnits", "preserveAspectRatio", "gradientUnits", "gradientTransform", "patternTransform"]);
  const attributeName = (key, svg) =>
    key === "className" ? "class" :
    svg && !CAMEL.has(key) ? key.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`) : key;
  const styleValue = (key, value) =>
    typeof value === "number" && !UNITLESS.has(key) ? `${value}px` : value == null || value === false ? "" : String(value);

  // A component is called where it appears, so a tree holds only elements
  // and text by the time it is rendered.
  const node = (type, props, ...children) => {
    props = props || {};
    if (typeof type === "function") return type({ ...props, children });
    return { type, props, children: children.flat(Infinity).filter((c) => c != null && c !== false && c !== true) };
  };

  const create = (vnode, svg) => {
    if (typeof vnode !== "object") return document.createTextNode(String(vnode));
    svg = svg || vnode.type === "svg";
    const element = svg ? document.createElementNS(SVG, vnode.type) : document.createElement(vnode.type);
    element.sindriType = vnode.type;
    element.sindriProps = {};
    element.sindriStyle = {};
    return element;
  };

  // Brings `dom` to what `vnode` describes, or replaces it, and returns the
  // element that now stands in its place.
  const patch = (parent, dom, vnode, svg) => {
    if (typeof vnode !== "object") {
      const text = String(vnode);
      if (dom && dom.nodeType === Node.TEXT_NODE) {
        if (dom.data !== text) dom.data = text;
        return dom;
      }
      const fresh = document.createTextNode(text);
      dom ? parent.replaceChild(fresh, dom) : parent.appendChild(fresh);
      return fresh;
    }
    svg = svg || vnode.type === "svg";
    if (!dom || dom.sindriType !== vnode.type) {
      const fresh = create(vnode, svg);
      dom ? parent.replaceChild(fresh, dom) : parent.appendChild(fresh);
      dom = fresh;
    }
    const before = dom.sindriProps;
    for (const [key, value] of Object.entries(vnode.props)) {
      if (key === "key" || key === "children" || key === "style") continue;
      if (before[key] === value) continue;
      const name = attributeName(key, svg);
      if (value == null || value === false) dom.removeAttribute(name);
      else dom.setAttribute(name, String(value));
    }
    for (const key of Object.keys(before)) {
      if (key === "key" || key === "children" || key === "style") continue;
      if (!(key in vnode.props)) dom.removeAttribute(attributeName(key, svg));
    }
    dom.sindriProps = vnode.props;
    const style = vnode.props.style || {};
    const was = dom.sindriStyle;
    for (const [key, value] of Object.entries(style)) {
      const text = styleValue(key, value);
      if (was[key] !== text) dom.style[key] = text;
    }
    for (const key of Object.keys(was)) if (!(key in style)) dom.style[key] = "";
    dom.sindriStyle = Object.fromEntries(Object.entries(style).map(([k, v]) => [k, styleValue(k, v)]));
    const children = dom.childNodes;
    vnode.children.forEach((child, i) => patch(dom, children[i], child, svg && vnode.type !== "foreignObject"));
    while (children.length > vnode.children.length) dom.removeChild(dom.lastChild);
    return dom;
  };

  // ---- The concepts, as designed. ----
  const Easing = {
    easeInOutCubic: t => t < 0.5 ? 4 * t * t * t : (t - 1) * (2 * t - 2) * (2 * t - 2) + 1
  };
  const INK = '#0d1117';
  const CREAM = '#e6e1d4';
  const GOLD = '#f0c050';
  const LINE = '#2a3038';
  const MUTED = '#8a8580';
  const frac = x => x - Math.floor(x);
  const clamp = (x, a = 0, b = 1) => Math.max(a, Math.min(b, x));
  const rnd = (i, k) => frac(Math.sin(i * 127.1 + k * 311.7) * 43758.5453);
  const easeOut = x => 1 - Math.pow(1 - clamp(x), 3);
  const since01 = (p, t, len) => {
    const d = p - t;
    return d >= 0 && d < len ? d / len : -1;
  };
  const inset = (S, c) => `inset 0 0 0 ${Math.max(1, 2 * S)}px ${c}`;
  const typed = (word, n) => node("span", null, word.slice(0, n), node("span", {
    style: {
      opacity: 0
    }
  }, word.slice(n)));
  const OUTER = '16,3 27.26,9.5 27.26,22.5 16,29 4.74,22.5 4.74,9.5';
  const INNER = '16,9 22.06,12.5 22.06,19.5 16,23 9.94,19.5 9.94,12.5';
  const CORE = '16,13.2 18.42,14.6 18.42,17.4 16,18.8 13.58,17.4 13.58,14.6';
  // ember -> orange -> gold -> white-gold
  const STOPS = [[0, [74, 28, 14]], [0.45, [212, 84, 30]], [0.78, [240, 192, 80]], [1, [255, 244, 214]]];
  function heatRGB(t) {
    t = clamp(t);
    for (let i = 1; i < STOPS.length; i++) {
      const [b, cb] = STOPS[i];
      const [a, ca] = STOPS[i - 1];
      if (t <= b) {
        const u = (t - a) / (b - a);
        return ca.map((c, j) => Math.round(c + (cb[j] - c) * u));
      }
    }
    return STOPS[STOPS.length - 1][1];
  }
  const rgb = (c, a = 1) => `rgba(${c[0]},${c[1]},${c[2]},${a})`;
  function Wordmark({
    S,
    font,
    iColor,
    iGlow,
    letterStyle,
    showNext,
    guides,
    hideTag,
    tag
  }) {
    const tg = tag || {};
    const letters = ['S', 'I', 'N', 'D', 'R', 'I'];
    const guide = top => node("div", {
      style: {
        position: 'absolute',
        left: '-30%',
        right: '-30%',
        top,
        height: Math.max(1, 2 * S),
        background: LINE,
        opacity: guides.op,
        transform: `scaleX(${guides.s})`
      }
    });
    return node("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'stretch',
        position: 'relative'
      }
    }, guides ? guide(0.06 * font) : null, guides ? guide(0.9 * font) : null, node("div", {
      style: {
        display: 'flex',
        fontFamily: "'Pixelify Sans', sans-serif",
        fontSize: font,
        fontWeight: 700,
        letterSpacing: `${0.14 * font}px`,
        lineHeight: 1,
        marginRight: -0.14 * font
      }
    }, letters.map((ch, i) => {
      const gold = i === 1 || i === 5;
      return node("span", {
        key: i,
        style: {
          color: gold ? iColor : CREAM,
          textShadow: gold && iGlow ? iGlow : 'none',
          display: 'inline-block',
          position: 'relative',
          ...(letterStyle ? letterStyle(i) : null)
        }
      }, ch);
    })), showNext ? node("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 0.27 * font,
        marginTop: 0.22 * font,
        opacity: hideTag ? 0 : 1
      }
    }, node("div", {
      style: {
        flex: 1,
        height: Math.max(1, 2 * S),
        background: tg.ruleColor || LINE,
        position: 'relative',
        transform: `scaleX(${tg.ruleScale ?? 1})`,
        transformOrigin: '0 50%',
        opacity: tg.ruleOp ?? 1
      }
    }, tg.fill ? node("div", {
      style: {
        position: 'absolute',
        left: 0,
        top: -Math.max(1, 1 * S),
        bottom: -Math.max(1, 1 * S),
        width: `${clamp(tg.fill) * 100}%`,
        background: tg.fillColor || GOLD
      }
    }) : null), node("div", {
      style: {
        position: 'relative'
      }
    }, tg.slot ? node("div", {
      style: {
        position: 'absolute',
        inset: 0,
        boxShadow: `inset 0 0 0 ${Math.max(1, 2 * S)}px ${LINE}`
      }
    }) : null, node("div", {
      style: {
        position: 'relative',
        fontFamily: "'JetBrains Mono', monospace",
        fontSize: 0.25 * font,
        color: INK,
        background: CREAM,
        padding: `${0.07 * font}px ${0.18 * font}px`,
        letterSpacing: '0.14em',
        whiteSpace: 'pre',
        ...tg.chip
      }
    }, tg.text ?? 'ENGINE'))) : null);
  }
  function Progress({
    p,
    S,
    w,
    color,
    label
  }) {
    const width = Math.min(w * 0.6, 560 * S);
    const seg = width * 0.3;
    const x = -seg + frac(p * 2) * (width + seg);
    return node("div", {
      style: {
        width,
        display: 'flex',
        flexDirection: 'column',
        gap: 18 * S
      }
    }, node("div", {
      style: {
        height: Math.max(2, 4 * S),
        background: '#1b2028',
        position: 'relative',
        overflow: 'hidden'
      }
    }, node("div", {
      style: {
        position: 'absolute',
        top: 0,
        bottom: 0,
        left: x,
        width: seg,
        background: `linear-gradient(90deg, ${color}00, ${color}, ${color}00)`
      }
    })), label ? node("div", {
      style: {
        fontFamily: "'JetBrains Mono', monospace",
        fontSize: 20 * S,
        letterSpacing: '0.24em',
        textTransform: 'uppercase',
        color: MUTED,
        textAlign: 'center'
      }
    }, label) : null);
  }

  // ---------- Forge: two hammer strikes per loop; the core heats, sparks fly, it cools
  const STRIKE = 0.05;
  function heatAt(q) {
    const x = q < STRIKE ? q / STRIKE : 1 - (q - STRIKE) / (1 - STRIKE);
    return 0.12 + 0.88 * Math.pow(x, q < STRIKE ? 1.5 : 2.4);
  }
  const impulseAt = q => q >= STRIKE ? Math.exp(-(q - STRIKE) * 16) : 0;
  function ForgeMark({
    p,
    M
  }) {
    const half = Math.floor(p * 3) % 3;
    const q = frac(p * 3);
    const heat = heatAt(q);
    const imp = impulseAt(q);
    const hc = heatRGB(0.5 + 0.5 * heat);
    const outerC = heatRGB(0.4 + 0.6 * heat);
    const outerMix = 0.65 * imp;
    const outerStroke = rgb([0, 1, 2].map(j => Math.round([230, 225, 212][j] * (1 - outerMix) + outerC[j] * outerMix)));
    const squash = 1 - 0.07 * imp;
    const e = easeOut((q - STRIKE) / 0.55);
    const waveOn = q >= STRIKE && q < STRIKE + 0.55;
    const sparks = [];
    if (q >= STRIKE) {
      const tau = q - STRIKE;
      for (let i = 0; i < 26; i++) {
        const id = i + half * 61;
        const life = 0.28 + 0.34 * rnd(id, 3);
        const t = tau / life;
        if (t >= 1) continue;
        const a = -Math.PI / 2 + (rnd(id, 1) - 0.5) * Math.PI * 1.7;
        const v = 7 + 12 * rnd(id, 2);
        const pos = tt => {
          const r = 11.5 + v * tt * 2.2;
          return [16 + Math.cos(a) * r, 16 + Math.sin(a) * r + 70 * tt * tt];
        };
        const [x1, y1] = pos(tau);
        const [x0, y0] = pos(Math.max(0, tau - 0.035));
        sparks.push(node("line", {
          key: i,
          x1: x0,
          y1: y0,
          x2: x1,
          y2: y1,
          stroke: rgb(heatRGB(1 - t * 0.75)),
          strokeWidth: 0.32 + 0.25 * rnd(id, 5),
          strokeLinecap: "round",
          opacity: 1 - t
        }));
      }
    }
    return node("div", {
      style: {
        position: 'relative',
        width: M,
        height: M,
        flex: 'none'
      }
    }, node("div", {
      style: {
        position: 'absolute',
        left: -M * 0.9,
        top: -M * 0.9,
        width: M * 2.8,
        height: M * 2.8,
        background: `radial-gradient(closest-side, ${rgb(hc, 0.22 * heat)}, ${rgb(hc, 0)})`
      }
    }), node("svg", {
      width: M,
      height: M,
      viewBox: "0 0 32 32",
      fill: "none",
      style: {
        position: 'absolute',
        inset: 0,
        overflow: 'visible',
        filter: `drop-shadow(0 0 ${M * (0.01 + 0.05 * heat)}px ${rgb(hc, 0.55 * heat)})`
      }
    }, node("polygon", {
      points: OUTER,
      stroke: outerStroke,
      strokeWidth: "1.6"
    }), node("g", {
      transform: `translate(16 16) scale(${squash}) translate(-16 -16)`
    }, node("polygon", {
      points: INNER,
      stroke: CREAM,
      strokeWidth: "1.2"
    }), node("polygon", {
      points: CORE,
      fill: rgb(hc)
    }))), node("svg", {
      width: M * 2,
      height: M * 2,
      viewBox: "-16 -16 64 64",
      style: {
        position: 'absolute',
        left: -M / 2,
        top: -M / 2,
        overflow: 'visible',
        pointerEvents: 'none'
      }
    }, waveOn ? node("g", {
      transform: `translate(16 16) scale(${1 + 0.8 * e}) translate(-16 -16)`
    }, node("polygon", {
      points: OUTER,
      fill: "none",
      stroke: rgb(outerC),
      strokeWidth: 0.5 / (1 + 0.8 * e),
      opacity: 0.55 * (1 - e)
    })) : null, sparks));
  }
  function Forge({
    p,
    S,
    w,
    h,
    horizontal,
    showNext,
    showProgress,
    label
  }) {
    const q = frac(p * 3);
    const heat = heatAt(q);
    const ic = heatRGB(0.74 + 0.26 * heat);
    const M = (horizontal ? 420 : 470) * S;
    const font = (horizontal ? 150 : 124) * S;
    const imp = impulseAt(q);
    const tag = {
      ruleColor: rgb(heatRGB(0.35 + 0.6 * heat), 0.25 + 0.75 * heat),
      chip: {
        background: imp > 0.03 ? rgb(heatRGB(0.82 + 0.18 * imp)) : CREAM,
        transform: `translateY(${-imp * 0.09 * font}px)`,
        boxShadow: `0 0 ${font * 0.3 * heat}px ${rgb(heatRGB(0.8), 0.6 * heat)}`
      }
    };
    return node(Layout, {
      S: S,
      horizontal: horizontal,
      showProgress: showProgress,
      mark: node(ForgeMark, {
        p: p,
        M: M
      }),
      word: node(Wordmark, {
        S: S,
        font: font,
        iColor: rgb(ic),
        iGlow: `0 0 ${font * 0.35 * heat}px ${rgb(ic, 0.8 * heat)}`,
        showNext: showNext,
        tag: tag
      }),
      progress: node(Progress, {
        p: p,
        S: S,
        w: w,
        color: rgb(heatRGB(0.8)),
        label: label
      })
    });
  }

  // ---------- Pixel: the mark stacks up block by block, holds, crumbles, rebuilds
  const N = 26;
  const CS = 32 / N;
  const CELLS = (() => {
    const out = [];
    for (let gy = 0; gy < N; gy++) {
      for (let gx = 0; gx < N; gx++) {
        const x = (gx + 0.5) * CS - 16;
        const y = (gy + 0.5) * CS - 16;
        const m = Math.max(Math.abs(x), 0.5 * Math.abs(x) + 0.866 * Math.abs(y));
        let kind = null;
        if (m >= 10.1 && m <= 12.1) kind = 'outer';else if (m >= 5.3 && m <= 6.9) kind = 'inner';else if (m <= 2.7) kind = 'core';
        if (!kind) continue;
        const i = out.length;
        out.push({
          gx,
          gy,
          kind,
          land: 0.1 + 0.036 * Math.floor((N - 1 - gy) / 3) + 0.008 * rnd(i, 1),
          drop: 0.79 + 0.13 * rnd(i, 4),
          drift: rnd(i, 6) - 0.5,
          tone: 0.86 + 0.14 * rnd(i, 7)
        });
      }
    }
    return out;
  })();
  function PixelMark({
    p,
    M
  }) {
    const FALL = 0.06;
    const band = (p - 0.44) / 0.3;
    const rects = CELLS.map((c, i) => {
      let dy = 0;
      let dx = 0;
      let op = 0;
      if (p < c.land) {
        const t = (p - (c.land - FALL)) / FALL;
        if (t <= 0) return null;
        dy = -(1 - easeOut(t)) * 10;
        op = clamp(t * 3);
      } else if (p < c.drop) {
        op = 1;
      } else {
        const tau = p - c.drop;
        const t = tau / 0.08;
        if (t >= 1) return null;
        dy = 900 * tau * tau;
        dx = c.drift * 40 * tau;
        op = 1 - t;
      }
      const u = (c.gx + c.gy) / (2 * N);
      const shine = p > 0.44 && p < 0.74 ? Math.max(0, 1 - Math.abs(u - band) * 9) : 0;
      let fill;
      if (c.kind === 'core') fill = rgb(heatRGB(0.78 + 0.22 * shine));else fill = rgb([230, 225, 212], (c.kind === 'inner' ? 0.82 : 1) * (c.tone + (1 - c.tone) * shine));
      return node("rect", {
        key: i,
        x: c.gx * CS + dx + CS * 0.07,
        y: c.gy * CS + dy + CS * 0.07,
        width: CS * 0.86,
        height: CS * 0.86,
        fill: fill,
        opacity: op
      });
    });
    return node("svg", {
      width: M,
      height: M,
      viewBox: "0 0 32 32",
      style: {
        display: 'block',
        overflow: 'visible',
        flex: 'none'
      }
    }, rects);
  }
  function Pixel({
    p,
    S,
    w,
    h,
    horizontal,
    showNext,
    showProgress,
    label
  }) {
    const M = (horizontal ? 440 : 480) * S;
    const font = (horizontal ? 150 : 124) * S;
    const letterStyle = i => {
      const on = 0.42 + 0.036 * i;
      const off = 0.8 + 0.015 * i;
      if (p < on || p >= off) return {
        opacity: 0.22
      };
      const t = clamp((p - on) / 0.05);
      return {
        opacity: 1,
        transform: `translateY(${-Math.sin(Math.PI * t) * 0.18 * font}px)`
      };
    };
    const n = p < 0.8 ? clamp(Math.floor((p - 0.62) / 0.025) + 1, 0, 6) : 0;
    const tag = {
      ruleScale: p < 0.8 ? Math.floor(clamp((p - 0.42) / 0.2) * 8) / 8 : 1 - Math.floor(clamp((p - 0.8) / 0.1) * 8) / 8,
      text: typed('ENGINE', n),
      chip: {
        opacity: n > 0 ? 1 : 0.22
      }
    };
    return node(Layout, {
      S: S,
      horizontal: horizontal,
      showProgress: showProgress,
      mark: node(PixelMark, {
        p: p,
        M: M
      }),
      word: node(Wordmark, {
        S: S,
        font: font,
        iColor: GOLD,
        letterStyle: letterStyle,
        showNext: showNext,
        tag: tag
      }),
      progress: node(Progress, {
        p: p,
        S: S,
        w: w,
        color: GOLD,
        label: label
      })
    });
  }

  // ---------- Blueprint: the mark is drafted edge by edge, inked, then wiped
  const OUT_V = [[16, 3], [27.26, 9.5], [27.26, 22.5], [16, 29], [4.74, 22.5], [4.74, 9.5]];
  const IN_V = [[16, 9], [22.06, 12.5], [22.06, 19.5], [16, 23], [9.94, 19.5], [9.94, 12.5]];
  const stepped = (t, n) => {
    t = clamp(t);
    if (t >= 1) return 1;
    const k = Math.floor(t * n);
    return (k + Easing.easeInOutCubic(t * n - k)) / n;
  };
  const along = (V, t) => {
    const f = t * V.length;
    const k = Math.min(V.length - 1, Math.floor(f));
    const u = f - k;
    const a = V[k];
    const b = V[(k + 1) % V.length];
    return [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u];
  };
  function BlueprintMark({
    p,
    M
  }) {
    const cOp = 1 - clamp((p - 0.62) / 0.1);
    const wipe = 1 - clamp((p - 0.88) / 0.08);
    const cross = easeOut(p / 0.12);
    const circ = easeOut((p - 0.03) / 0.14);
    const tO = stepped((p - 0.1) / 0.3, 6);
    const tI = stepped((p - 0.36) / 0.18, 6);
    const core = easeOut((p - 0.54) / 0.06);
    const dim = easeOut((p - 0.42) / 0.12);
    const pen = p > 0.1 && p < 0.4 ? along(OUT_V, tO) : p > 0.36 && p < 0.54 ? along(IN_V, tI) : null;
    const sw = 0.18;
    return node("svg", {
      width: M,
      height: M,
      viewBox: "0 0 32 32",
      fill: "none",
      style: {
        display: 'block',
        overflow: 'visible',
        flex: 'none',
        opacity: wipe
      }
    }, node("g", {
      stroke: MUTED,
      strokeWidth: sw,
      opacity: cOp * 0.8
    }, node("line", {
      x1: 16 - 19 * cross,
      y1: "16",
      x2: 16 + 19 * cross,
      y2: "16"
    }), node("line", {
      x1: "16",
      y1: 16 - 19 * cross,
      x2: "16",
      y2: 16 + 19 * cross
    }), node("circle", {
      cx: "16",
      cy: "16",
      r: "13",
      strokeDasharray: "81.7",
      strokeDashoffset: 81.7 * (1 - circ),
      transform: "rotate(-90 16 16)"
    }), node("circle", {
      cx: "16",
      cy: "16",
      r: "7",
      strokeDasharray: "44",
      strokeDashoffset: 44 * (1 - easeOut((p - 0.3) / 0.1)),
      transform: "rotate(-90 16 16)"
    })), node("polygon", {
      points: OUTER,
      stroke: CREAM,
      strokeWidth: "1.6",
      strokeLinejoin: "miter",
      strokeDasharray: "78",
      strokeDashoffset: 78 * (1 - tO)
    }), node("polygon", {
      points: INNER,
      stroke: CREAM,
      strokeWidth: "1.2",
      strokeDasharray: "42",
      strokeDashoffset: 42 * (1 - tI)
    }), node("g", {
      transform: `translate(16 16) scale(${core}) translate(-16 -16)`
    }, node("polygon", {
      points: CORE,
      fill: GOLD
    })), node("g", {
      opacity: cOp
    }, OUT_V.map((v, i) => tO * 6 >= i ? node("rect", {
      key: i,
      x: v[0] - 0.45,
      y: v[1] - 0.45,
      width: "0.9",
      height: "0.9",
      fill: INK,
      stroke: GOLD,
      strokeWidth: sw
    }) : null), node("g", {
      stroke: MUTED,
      strokeWidth: sw,
      opacity: dim
    }, node("line", {
      x1: "4.74",
      y1: "32",
      x2: 4.74 + 22.52 * dim,
      y2: "32"
    }), node("line", {
      x1: "4.74",
      y1: "31",
      x2: "4.74",
      y2: "33"
    }), node("line", {
      x1: 4.74 + 22.52 * dim,
      y1: "31",
      x2: 4.74 + 22.52 * dim,
      y2: "33"
    })), node("text", {
      x: "16",
      y: "34.8",
      textAnchor: "middle",
      fill: MUTED,
      opacity: dim,
      style: {
        fontFamily: "'JetBrains Mono', monospace",
        fontSize: 1.3,
        letterSpacing: '0.1em'
      }
    }, "22.52"), pen ? node("circle", {
      cx: pen[0],
      cy: pen[1],
      r: "0.7",
      fill: GOLD
    }) : null));
  }
  function Blueprint({
    p,
    S,
    w,
    h,
    horizontal,
    showNext,
    showProgress,
    label
  }) {
    const M = (horizontal ? 420 : 470) * S;
    const font = (horizontal ? 150 : 124) * S;
    const cOp = 1 - clamp((p - 0.62) / 0.1);
    const letterStyle = i => {
      const on = 0.56 + 0.025 * i;
      return {
        opacity: p >= on && p < 0.92 ? 1 : 0.2
      };
    };
    const inked = p >= 0.64 && p < 0.92;
    const tag = {
      ruleScale: p < 0.92 ? easeOut((p - 0.5) / 0.1) : 1 - clamp((p - 0.92) / 0.06),
      chip: {
        background: inked ? CREAM : 'transparent',
        color: inked ? INK : CREAM,
        boxShadow: inset(S, CREAM),
        opacity: p >= 0.58 && p < 0.95 ? 1 : 0.2
      }
    };
    return node(Layout, {
      S: S,
      horizontal: horizontal,
      showProgress: showProgress,
      grid: S,
      mark: node(BlueprintMark, {
        p: p,
        M: M
      }),
      word: node(Wordmark, {
        S: S,
        font: font,
        iColor: GOLD,
        letterStyle: letterStyle,
        showNext: showNext,
        tag: tag,
        guides: {
          op: cOp,
          s: easeOut((p - 0.04) / 0.2)
        }
      }),
      progress: node(Progress, {
        p: p,
        S: S,
        w: w,
        color: GOLD,
        label: label
      })
    });
  }

  // ---------- Cube: the hex is an isometric cube; it tumbles 90° twice and lands back on itself
  const SQ2 = Math.SQRT2,
    SQ3 = Math.sqrt(3),
    SQ6 = Math.sqrt(6);
  const ISO_R = 4 / SQ6;
  const CUBE_V = [];
  for (let i = 0; i < 8; i++) CUBE_V.push([i & 1 ? 1 : -1, i & 2 ? 1 : -1, i & 4 ? 1 : -1]);
  const CUBE_E = [];
  for (let i = 0; i < 8; i++) for (let b = 0; b < 3; b++) {
    const j = i ^ 1 << b;
    if (j > i) CUBE_E.push([i, j, b]);
  }
  const rotAxis = (v, ax, a) => {
    const c = Math.cos(a),
      sn = Math.sin(a);
    const [x, y, z] = v;
    if (ax === 0) return [x, y * c - z * sn, y * sn + z * c];
    if (ax === 1) return [x * c + z * sn, y, -x * sn + z * c];
    return [x * c - y * sn, x * sn + y * c, z];
  };
  const isoDepth = v => (v[0] + v[1] + v[2]) / SQ3;
  const isoXY = (v, k) => [16 + k * (v[0] - v[1]) / SQ2, 16 - k * (-v[0] - v[1] + 2 * v[2]) / SQ6];
  function easeInOutBack(x) {
    const c = 1.70158 * 1.525;
    return x < 0.5 ? Math.pow(2 * x, 2) * ((c + 1) * 2 * x - c) / 2 : (Math.pow(2 * x - 2, 2) * ((c + 1) * (x * 2 - 2) + c) + 2) / 2;
  }
  function hull(pts) {
    const P = pts.slice().sort((a, b) => a[0] - b[0] || a[1] - b[1]);
    const cr = (o, a, b) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    const lo = [],
      up = [];
    for (const p of P) {
      while (lo.length >= 2 && cr(lo[lo.length - 2], lo[lo.length - 1], p) <= 0) lo.pop();
      lo.push(p);
    }
    for (let i = P.length - 1; i >= 0; i--) {
      const p = P[i];
      while (up.length >= 2 && cr(up[up.length - 2], up[up.length - 1], p) <= 0) up.pop();
      up.push(p);
    }
    return lo.slice(0, -1).concat(up.slice(0, -1));
  }
  const TUMBLES = [[0.08, 0.3, 0], [0.56, 0.3, 1]];
  function tumbleAt(p, delay) {
    for (const [st, dur, ax] of TUMBLES) {
      const t = (p - st - delay) / dur;
      if (t > 0 && t < 1) return {
        ax,
        a: Math.PI / 2 * easeInOutBack(t),
        m: Math.sin(Math.PI * t),
        t
      };
    }
    return {
      ax: 0,
      a: 0,
      m: 0,
      t: 0
    };
  }
  function cubeEdges(k, rot, sw, key) {
    const V = CUBE_V.map(v => rotAxis(v, rot.ax, rot.a));
    const faceFront = (axis, sign) => {
      const n = [0, 0, 0];
      n[axis] = sign;
      return isoDepth(rotAxis(n, rot.ax, rot.a)) > 0;
    };
    const items = CUBE_E.map(([i, j, b], n) => {
      const others = [0, 1, 2].filter(q => q !== b);
      const f1 = faceFront(others[0], CUBE_V[i][others[0]]);
      const f2 = faceFront(others[1], CUBE_V[i][others[1]]);
      const sil = f1 !== f2;
      const front = f1 && f2;
      const [x1, y1] = isoXY(V[i], k);
      const [x2, y2] = isoXY(V[j], k);
      const op = sil ? 1 : front ? 0.9 * rot.m : 0.28 * rot.m;
      return {
        n,
        x1,
        y1,
        x2,
        y2,
        op,
        z: sil ? 1 : front ? 2 : 0
      };
    }).sort((a, b) => a.z - b.z);
    return items.map(e => e.op > 0.01 ? node("line", {
      key: key + e.n,
      x1: e.x1,
      y1: e.y1,
      x2: e.x2,
      y2: e.y2,
      stroke: CREAM,
      strokeWidth: sw,
      strokeLinecap: "round",
      opacity: e.op
    }) : null);
  }
  function CubeMark({
    p,
    M
  }) {
    const outer = tumbleAt(p, 0);
    const inner = tumbleAt(p, 0.05);
    const innerRot = {
      ...inner,
      ax: inner.ax === 0 ? 2 : 0,
      a: -inner.a
    };
    const coreV = CUBE_V.map(v => isoXY(rotAxis(v, outer.ax, outer.a), 2.8 / ISO_R));
    const hop = -Math.sin(Math.PI * outer.t) * 0.09 * M;
    let land = 0;
    for (const [st, dur] of TUMBLES) {
      const d = p - (st + dur);
      if (d > 0 && d < 0.07) land = Math.sin(Math.PI * d / 0.07);
    }
    const flash = land;
    return node("div", {
      style: {
        position: 'relative',
        width: M,
        height: M,
        flex: 'none'
      }
    }, node("div", {
      style: {
        position: 'absolute',
        left: M * 0.15,
        right: M * 0.15,
        bottom: -M * 0.08,
        height: M * 0.08,
        background: `radial-gradient(closest-side, rgba(240,192,80,${0.16 * (1 - outer.m * 0.7)}), rgba(240,192,80,0))`,
        transform: `scaleX(${1 - 0.35 * outer.m + 0.12 * land})`
      }
    }), node("svg", {
      width: M,
      height: M,
      viewBox: "0 0 32 32",
      fill: "none",
      style: {
        position: 'absolute',
        inset: 0,
        overflow: 'visible',
        transform: `translateY(${hop}px) scale(${1 + 0.04 * land}, ${1 - 0.05 * land})`,
        transformOrigin: '50% 100%'
      }
    }, cubeEdges(13 / ISO_R, outer, 1.6, 'o'), cubeEdges(7 / ISO_R, innerRot, 1.2, 'i'), node("polygon", {
      points: hull(coreV).map(q => q.join(',')).join(' '),
      fill: rgb(heatRGB(0.78 + 0.22 * flash))
    })));
  }
  function Cube({
    p,
    S,
    w,
    h,
    horizontal,
    showNext,
    showProgress,
    label
  }) {
    const M = (horizontal ? 420 : 470) * S;
    const font = (horizontal ? 150 : 124) * S;
    const letterStyle = i => {
      let y = 0;
      for (const [st, dur] of TUMBLES) {
        const d = frac(p - (st + dur + 0.02 + 0.022 * i));
        if (d < 0.06) y = -Math.sin(Math.PI * d / 0.06) * 0.12 * font;
      }
      return {
        transform: `translateY(${y}px)`
      };
    };
    const flip = tumbleAt(p, 0.04);
    const tag = {
      chip: {
        transform: `scaleY(${flip.m > 0 ? Math.cos(2 * Math.PI * Easing.easeInOutCubic(flip.t)) : 1})`
      }
    };
    return node(Layout, {
      S: S,
      horizontal: horizontal,
      showProgress: showProgress,
      mark: node(CubeMark, {
        p: p,
        M: M
      }),
      word: node(Wordmark, {
        S: S,
        font: font,
        iColor: GOLD,
        letterStyle: letterStyle,
        showNext: showNext,
        tag: tag
      }),
      progress: node(Progress, {
        p: p,
        S: S,
        w: w,
        color: GOLD,
        label: label
      })
    });
  }

  // ---------- Arcade: a CRT powers on, the logo glitches on the beat, the tube switches off
  function ArcadeMark({
    p,
    M
  }) {
    const beat = frac(p * 4);
    const blink = beat < 0.5 ? 1 : 0.35;
    return node("svg", {
      width: M,
      height: M,
      viewBox: "0 0 32 32",
      fill: "none",
      style: {
        display: 'block',
        flex: 'none',
        overflow: 'visible'
      }
    }, node("polygon", {
      points: OUTER,
      stroke: CREAM,
      strokeWidth: "1.6"
    }), node("polygon", {
      points: INNER,
      stroke: CREAM,
      strokeWidth: "1.2"
    }), node("polygon", {
      points: CORE,
      fill: GOLD,
      opacity: blink
    }));
  }
  function Arcade({
    p,
    S,
    w,
    h,
    horizontal,
    showNext,
    showProgress,
    label
  }) {
    const M = (horizontal ? 420 : 470) * S;
    const font = (horizontal ? 150 : 124) * S;
    let sy = 1,
      bright = 1,
      show = true,
      beamW = 0,
      beamH = 0,
      beamOp = 0;
    if (p < 0.1) {
      show = false;
      if (p < 0.04) {
        const t = easeOut(p / 0.04);
        beamW = beamH = 18 * S * t;
        beamOp = t;
      } else {
        const t = easeOut((p - 0.04) / 0.06);
        beamW = 18 * S + (w * 0.86 - 18 * S) * t;
        beamH = 18 * S - 12 * S * t;
        beamOp = 1;
      }
    } else if (p < 0.17) {
      const t = easeOut((p - 0.1) / 0.07);
      sy = 0.01 + 0.99 * t;
      bright = 3 - 2 * t;
    } else if (p >= 0.82 && p < 0.88) {
      const t = clamp((p - 0.82) / 0.06);
      sy = 1 - 0.99 * t * t;
      bright = 1 + 2 * t;
    } else if (p >= 0.88) {
      show = false;
      if (p < 0.94) {
        const t = Easing.easeInOutCubic((p - 0.88) / 0.06);
        beamW = w * 0.86 * (1 - t) + 14 * S * t;
        beamH = 6 * S + 8 * S * t;
        beamOp = 1;
      } else {
        const t = clamp((p - 0.94) / 0.06);
        beamW = beamH = 14 * S * (1 - t);
        beamOp = 1 - t;
      }
    }
    const glitch = p > 0.44 && p < 0.48 || p > 0.64 && p < 0.665 ? 1 : 0;
    const step = Math.floor(p * 140);
    const boot = p >= 0.1 && p < 0.3 ? 1 - (p - 0.1) / 0.2 : p >= 0.8 ? clamp((p - 0.8) / 0.08) : 0;
    const split = S * (3 + 18 * glitch + 16 * boot);
    const jx = glitch ? (rnd(step, 1) - 0.5) * 50 * S : 0;
    const sliceTop = 20 + 50 * rnd(step, 2);
    const roll = frac(p * 2) * 130 - 15;
    return node("div", {
      style: {
        position: 'absolute',
        inset: 0,
        background: INK,
        overflow: 'hidden'
      }
    }, node(Layout, {
      S: S,
      horizontal: horizontal,
      showProgress: showProgress,
      contentStyle: {
        opacity: show ? 1 : 0,
        transform: `translateX(${jx}px) scaleY(${sy})`,
        filter: `brightness(${bright}) drop-shadow(${-split}px 0 0 rgba(255,40,90,0.7)) drop-shadow(${split}px 0 0 rgba(40,220,255,0.6))`
      },
      ghost: glitch && show ? {
        clipPath: `inset(${sliceTop}% 0 ${100 - sliceTop - 7 - 6 * rnd(step, 3)}% 0)`,
        transform: `translateX(${(rnd(step, 4) - 0.5) * 120 * S}px)`
      } : null,
      mark: node(ArcadeMark, {
        p: p,
        M: M
      }),
      word: node(Wordmark, {
        S: S,
        font: font,
        iColor: GOLD,
        showNext: showNext,
        tag: {
          chip: frac(p * 4) < 0.5 ? {} : {
            background: 'transparent',
            color: GOLD,
            boxShadow: inset(S, GOLD)
          }
        }
      }),
      progress: node(Progress, {
        p: p,
        S: S,
        w: w,
        color: GOLD,
        label: label
      })
    }), node("div", {
      style: {
        position: 'absolute',
        left: '50%',
        top: '50%',
        width: beamW,
        height: beamH,
        marginLeft: -beamW / 2,
        marginTop: -beamH / 2,
        background: '#fff7e0',
        opacity: beamOp,
        boxShadow: `0 0 ${30 * S}px ${8 * S}px rgba(255,240,200,0.5)`,
        borderRadius: beamH
      }
    }), node("div", {
      style: {
        position: 'absolute',
        inset: 0,
        pointerEvents: 'none',
        backgroundImage: `repeating-linear-gradient(0deg, rgba(0,0,0,0.32) 0px, rgba(0,0,0,0.32) ${Math.max(1, 2 * S)}px, rgba(0,0,0,0) ${Math.max(1, 2 * S)}px, rgba(0,0,0,0) ${Math.max(2, 5 * S)}px)`
      }
    }), node("div", {
      style: {
        position: 'absolute',
        left: 0,
        right: 0,
        top: `${roll}%`,
        height: '14%',
        pointerEvents: 'none',
        background: 'linear-gradient(180deg, rgba(230,225,212,0), rgba(230,225,212,0.035), rgba(230,225,212,0))'
      }
    }), node("div", {
      style: {
        position: 'absolute',
        inset: 0,
        pointerEvents: 'none',
        background: 'radial-gradient(120% 110% at 50% 50%, rgba(0,0,0,0) 55%, rgba(0,0,0,0.6) 100%)'
      }
    }));
  }

  // ---------- Diorama: the mark is a hex tile on an isometric map; the map builds around it, the core pistons up, the map falls away
  const HEX_R = 1;
  const hexCorners = (cx, cy, r) => [0, 1, 2, 3, 4, 5].map(i => {
    const a = Math.PI / 6 + i * Math.PI / 3;
    return [cx + r * Math.cos(a), cy + r * Math.sin(a)];
  });
  const HEX_C = 2.2;
  const MAP = (() => {
    const out = [];
    for (let k = 0; k < 6; k++) {
      const a1 = k * Math.PI / 3;
      out.push({
        x: 3.0 * Math.cos(a1),
        y: 3.0 * Math.sin(a1),
        r: 0.96
      });
      const a2 = Math.PI / 6 + k * Math.PI / 3;
      out.push({
        x: 4.25 * Math.cos(a2),
        y: 4.25 * Math.sin(a2),
        r: 0.72
      });
    }
    return out.map((t, i) => ({
      ...t,
      i,
      h: 0.2 + 0.5 * rnd(i, 9),
      land: 0.05 + 0.028 * i,
      fall: 0.78 + 0.14 * rnd(i, 10),
      depth: t.x + t.y
    }));
  })();
  const isoP = (x, y, z) => [(x - y) * 0.866, (x + y) * 0.5 - z];
  const ptsStr = P => P.map(q => q[0].toFixed(3) + ',' + q[1].toFixed(3)).join(' ');
  const shadeSide = (a, base) => {
    const lit = 0.5 + 0.5 * Math.max(0, Math.cos(a - Math.PI * 0.85));
    return rgb(base.map(c => Math.round(c * (0.55 + 0.6 * lit))));
  };
  function prism(key, cx, cy, z0, h, r, topFill, topStroke, sideBase, strokeW, opacity) {
    const C = hexCorners(cx, cy, r);
    const sides = [];
    for (let i = 0; i < 6; i++) {
      const a = C[i],
        b = C[(i + 1) % 6];
      const nA = Math.PI / 3 + i * Math.PI / 3;
      const nx = Math.cos(nA),
        ny = Math.sin(nA);
      if (nx + ny <= 0.01) continue;
      const quad = [isoP(a[0], a[1], z0), isoP(b[0], b[1], z0), isoP(b[0], b[1], z0 + h), isoP(a[0], a[1], z0 + h)];
      sides.push(node("polygon", {
        key: key + 's' + i,
        points: ptsStr(quad),
        fill: shadeSide(nA, sideBase),
        stroke: rgb(sideBase.map(c => Math.round(c * 0.6))),
        strokeWidth: strokeW * 0.6,
        strokeLinejoin: "round"
      }));
    }
    const top = C.map(c => isoP(c[0], c[1], z0 + h));
    return node("g", {
      key: key,
      opacity: opacity
    }, sides, node("polygon", {
      points: ptsStr(top),
      fill: topFill,
      stroke: topStroke,
      strokeWidth: strokeW,
      strokeLinejoin: "round"
    }));
  }
  function DioramaMark({
    p,
    M
  }) {
    const tiles = MAP.map(t => {
      let dz = 0,
        op = 1;
      if (p < t.land) {
        const u = (p - (t.land - 0.07)) / 0.07;
        if (u <= 0) return null;
        dz = 7 * Math.pow(1 - u, 2);
        op = clamp(u * 2.5);
      } else if (p < t.land + 0.05) {
        dz = -0.08 * Math.sin(Math.PI * (p - t.land) / 0.05);
      } else if (p >= t.fall) {
        const tau = p - t.fall;
        const u = tau / 0.08;
        if (u >= 1) return null;
        dz = -260 * tau * tau;
        op = 1 - u;
      }
      return {
        t,
        dz,
        op
      };
    }).filter(Boolean);
    const coreUp = (() => {
      if (p < 0.46) return 0;
      if (p < 0.54) return easeInOutBack((p - 0.46) / 0.08);
      if (p < 0.74) return 1 + 0.06 * Math.sin((p - 0.54) / 0.1 * Math.PI);
      if (p < 0.8) return 1 - Easing.easeInOutCubic((p - 0.74) / 0.06);
      return 0;
    })();
    const pulse = clamp(coreUp);
    const H0 = 1.0;
    const back = tiles.filter(x => x.t.depth < 0).sort((a, b) => a.t.depth - b.t.depth);
    const front = tiles.filter(x => x.t.depth >= 0).sort((a, b) => a.t.depth - b.t.depth);
    const tileEl = ({
      t,
      dz,
      op
    }) => prism('t' + t.i, t.x, t.y, dz, t.h, t.r, '#1b2028', LINE, [27, 32, 40], 0.035, op);
    const outerTop = hexCorners(0, 0, HEX_C).map(c => isoP(c[0], c[1], H0));
    const innerTop = hexCorners(0, 0, HEX_C * (7 / 13)).map(c => isoP(c[0], c[1], H0));
    const coreR = HEX_C * (2.8 / 13);
    const coreH = 0.06 + 1.2 * coreUp;
    const glow = heatRGB(0.78 + 0.2 * pulse);
    return node("svg", {
      width: M,
      height: M * 0.72,
      viewBox: "-6.3 -5 12.6 9.4",
      fill: "none",
      style: {
        display: 'block',
        flex: 'none',
        overflow: 'visible'
      }
    }, node("ellipse", {
      cx: "0",
      cy: "0.2",
      rx: 5.4,
      ry: 3.1,
      fill: rgb(glow, 0.06 + 0.06 * pulse)
    }), back.map(tileEl), prism('center', 0, 0, 0, H0, HEX_C, '#12171e', CREAM, [42, 48, 56], 0.14, 1), node("polygon", {
      points: ptsStr(outerTop),
      stroke: CREAM,
      strokeWidth: "0.16",
      strokeLinejoin: "round"
    }), node("polygon", {
      points: ptsStr(innerTop),
      stroke: CREAM,
      strokeWidth: "0.12",
      strokeLinejoin: "round"
    }), node("g", {
      style: {
        filter: `drop-shadow(0 0 ${M * 0.02 * pulse}px ${rgb(glow, 0.8 * pulse)})`
      }
    }, prism('core', 0, 0, H0, coreH, coreR, rgb(glow), 'none', [200, 140, 40], 0.02, 1)), front.map(tileEl));
  }
  function Diorama({
    p,
    S,
    w,
    h,
    horizontal,
    showNext,
    showProgress,
    label
  }) {
    const M = (horizontal ? 560 : 640) * S;
    const font = (horizontal ? 140 : 124) * S;
    const letterStyle = i => {
      const d = frac(p - (0.54 + 0.02 * i));
      const y = d < 0.06 ? -Math.sin(Math.PI * d / 0.06) * 0.1 * font : 0;
      return {
        transform: `translateY(${y}px)`
      };
    };
    return node(Layout, {
      S: S,
      horizontal: horizontal,
      showProgress: showProgress,
      mark: node(DioramaMark, {
        p: p,
        M: M
      }),
      word: node(Wordmark, {
        S: S,
        font: font,
        iColor: GOLD,
        letterStyle: letterStyle,
        showNext: showNext,
        tag: (() => {
          if (p < 0.48 || p >= 0.88) return {
            slot: true,
            chip: {
              opacity: 0
            }
          };
          if (p < 0.56) {
            const u = (p - 0.48) / 0.08;
            return {
              slot: true,
              chip: {
                transform: `translateY(${-Math.pow(1 - u, 2) * 0.9 * font}px)`,
                opacity: clamp(u * 3)
              }
            };
          }
          if (p < 0.8) {
            const d = since01(p, 0.56, 0.04);
            return {
              slot: true,
              chip: {
                transform: `translateY(${d >= 0 ? -Math.sin(Math.PI * d) * 0.05 * font : 0}px)`
              }
            };
          }
          const u = (p - 0.8) / 0.08;
          return {
            slot: true,
            chip: {
              transform: `translateY(${u * u * 0.9 * font}px)`,
              opacity: 1 - u
            }
          };
        })()
      }),
      progress: node(Progress, {
        p: p,
        S: S,
        w: w,
        color: GOLD,
        label: label
      })
    });
  }
  function Layout({
    S,
    horizontal,
    mark,
    word,
    progress,
    showProgress,
    grid,
    contentStyle,
    ghost,
    bare
  }) {
    const cell = 60 * S;
    const content = (extra, key) => node("div", {
      key: key,
      style: {
        position: 'absolute',
        inset: 0,
        display: 'flex',
        flexDirection: horizontal ? 'row' : 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: (horizontal ? 76 : 64) * S,
        ...extra
      }
    }, mark, word);
    return node("div", {
      style: {
        position: 'absolute',
        inset: 0,
        background: bare ? 'transparent' : INK
      }
    }, grid ? node("div", {
      style: {
        position: 'absolute',
        inset: 0,
        backgroundImage: `linear-gradient(${LINE} 1px, transparent 1px), linear-gradient(90deg, ${LINE} 1px, transparent 1px)`,
        backgroundSize: `${cell}px ${cell}px`,
        backgroundPosition: 'center center',
        opacity: 0.4
      }
    }) : null, content(contentStyle, 'c'), ghost ? content({
      ...contentStyle,
      ...ghost,
      background: INK
    }, 'g') : null, showProgress ? node("div", {
      style: {
        position: 'absolute',
        left: 0,
        right: 0,
        bottom: 110 * S,
        display: 'flex',
        justifyContent: 'center'
      }
    }, progress) : null);
  }
  const EMBER = '#d4541e';
  const PANEL = '#12171e';
  const TILE = '#1b2028';
  const MONO = "'JetBrains Mono', monospace";
  const PIX = "'Pixelify Sans', sans-serif";
  const sizes = (S, horizontal) => ({
    M: (horizontal ? 420 : 470) * S,
    font: (horizontal ? 150 : 124) * S
  });
  const since = (p, t, len) => {
    const d = p - t;
    return d >= 0 && d < len ? d / len : -1;
  };
  function Logo({
    M,
    stroke = CREAM,
    coreFill = GOLD,
    coreOp = 1,
    style,
    children,
    before
  }) {
    return node("svg", {
      width: M,
      height: M,
      viewBox: "0 0 32 32",
      fill: "none",
      style: {
        display: 'block',
        flex: 'none',
        overflow: 'visible',
        ...style
      }
    }, before, node("polygon", {
      points: OUTER,
      stroke: stroke,
      strokeWidth: "1.6"
    }), node("polygon", {
      points: INNER,
      stroke: stroke,
      strokeWidth: "1.2"
    }), node("polygon", {
      points: CORE,
      fill: coreFill,
      opacity: coreOp
    }), children);
  }
  const hexRing = (scale, color, width, op, key) => node("g", {
    key: key,
    transform: `translate(16 16) scale(${scale}) translate(-16 -16)`
  }, node("polygon", {
    points: OUTER,
    stroke: color,
    strokeWidth: width / scale,
    opacity: op,
    fill: "none"
  }));
  const Base = props => node(Layout, {
    S: props.S,
    horizontal: props.horizontal,
    showProgress: props.showProgress,
    progress: node(Progress, {
      p: props.p,
      S: props.S,
      w: props.w,
      color: props.barColor || GOLD,
      label: props.label
    }),
    ...props.layout,
    mark: props.mark,
    word: props.word
  });

  // ---------- Boss: SINDRI is the boss. Four hits drain the bar, then it heals.
  const HITS = [0.14, 0.28, 0.42, 0.56];
  const DMG = ['312', '287', '451', '199'];
  function Boss(P) {
    const {
      p,
      S,
      horizontal,
      showNext
    } = P;
    const {
      M,
      font
    } = sizes(S, horizontal);
    let hp = 10 - 2 * HITS.filter(t => p >= t).length;
    if (p >= 0.7) hp = 2 + clamp(Math.floor((p - 0.7) / 0.024), 0, 8);
    let lastHit = -1;
    HITS.forEach(t => {
      if (p >= t) lastHit = t;
    });
    const d = lastHit >= 0 ? p - lastHit : 1;
    const amp = d < 0.08 ? (1 - d / 0.08) * 16 * S : 0;
    const step = Math.floor(p * 220);
    const shake = `translate(${(rnd(step, 1) - 0.5) * 2 * amp}px, ${(rnd(step, 2) - 0.5) * 2 * amp}px)`;
    const flash = d < 0.03;
    const segs = [];
    for (let k = 0; k < 10; k++) {
      const hitIdx = Math.floor((9 - k) / 2);
      const ghost = p < 0.7 && k >= hp && since(p, HITS[3 - hitIdx], 0.07) >= 0 && k < hp + 2;
      segs.push(node("div", {
        key: k,
        style: {
          flex: 1,
          background: k < hp ? GOLD : ghost ? CREAM : TILE,
          opacity: ghost ? 1 - since(p, HITS[3 - hitIdx], 0.07) : 1
        }
      }));
    }
    const pops = HITS.map((t, i) => {
      const u = since(p, t, 0.13);
      if (u < 0) return null;
      return node("div", {
        key: i,
        style: {
          position: 'absolute',
          left: M * (0.62 + 0.08 * (i % 2)),
          top: M * (0.02 + 0.06 * i) - u * M * 0.28,
          fontFamily: PIX,
          fontWeight: 700,
          fontSize: (i === 2 ? 92 : 68) * S,
          color: i === 2 ? EMBER : GOLD,
          opacity: 1 - u * u,
          transform: `scale(${1 + 0.4 * Math.max(0, 1 - u * 6)})`,
          textShadow: `0 ${4 * S}px 0 ${INK}`
        }
      }, i === 2 ? 'CRIT ' : '', DMG[i]);
    });
    return node(Base, {
      ...P,
      layout: {
        contentStyle: {
          transform: shake
        }
      },
      mark: node("div", {
        style: {
          position: 'relative',
          width: M,
          height: M,
          flex: 'none'
        }
      }, node(Logo, {
        M: M,
        stroke: flash ? '#ffffff' : CREAM,
        coreFill: flash ? '#ffffff' : GOLD,
        style: {
          filter: flash ? 'brightness(2)' : 'none',
          transform: `scale(${flash ? 0.96 : 1})`
        }
      }), pops),
      word: node("div", {
        style: {
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'stretch',
          gap: 30 * S
        }
      }, node(Wordmark, {
        S: S,
        font: font,
        iColor: GOLD,
        showNext: showNext,
        tag: {
          chip: d < 0.06 ? {
            background: EMBER,
            color: CREAM,
            transform: `translateX(${(rnd(step, 3) - 0.5) * 0.12 * font}px)`
          } : {}
        }
      }), node("div", {
        style: {
          display: 'flex',
          gap: 6 * S,
          height: 30 * S,
          padding: 5 * S,
          border: `${Math.max(1, 3 * S)}px solid ${LINE}`,
          background: INK
        }
      }, segs))
    });
  }

  // ---------- Loot: a hex crate drops, bursts, the core floats up in a beam, then it all packs away
  function Loot(P) {
    const {
      p,
      S,
      h,
      horizontal,
      showNext
    } = P;
    const {
      M,
      font
    } = sizes(S, horizontal);
    let y = 0,
      rot = 0,
      op = 1,
      sc = 1;
    if (p < 0.12) y = -h * 0.75 * Math.pow(1 - p / 0.12, 2);else if (p < 0.17) y = -M * 0.05 * Math.sin(Math.PI * (p - 0.12) / 0.05);
    if (p > 0.2 && p < 0.31) rot = Math.sin(2 * Math.PI * (p - 0.2) / 0.022) * 5 * Math.sin(Math.PI * (p - 0.2) / 0.11);
    if (p >= 0.86) {
      const u = clamp((p - 0.86) / 0.12);
      op = 1 - u;
      sc = 1 - 0.15 * u;
    }
    let b = 0;
    if (p >= 0.32 && p < 0.76) b = easeOut((p - 0.32) / 0.08);else if (p >= 0.76 && p < 0.84) b = 1 - Easing.easeInOutCubic((p - 0.76) / 0.08);
    const edges = OUT_V.map((a, i) => {
      const c = OUT_V[(i + 1) % 6];
      const mx = (a[0] + c[0]) / 2 - 16,
        my = (a[1] + c[1]) / 2 - 16;
      const L = Math.hypot(mx, my);
      const o = b * 3.2;
      return node("line", {
        key: i,
        x1: a[0] + mx / L * o,
        y1: a[1] + my / L * o,
        x2: c[0] + mx / L * o,
        y2: c[1] + my / L * o,
        stroke: CREAM,
        strokeWidth: "1.6",
        strokeLinecap: "round"
      });
    });
    const bob = 0.6 * Math.sin(2 * Math.PI * frac(p * 4)) * b;
    const coreY = -b * 10 + bob;
    const stars = [0, 1, 2, 3, 4, 5, 6, 7].map(i => {
      const tw = Math.pow(Math.max(0, Math.sin(2 * Math.PI * (p * 3 + rnd(i, 1)))), 3) * b;
      const x = 16 + (rnd(i, 2) - 0.5) * 18,
        yy = 16 + coreY + (rnd(i, 3) - 0.5) * 14;
      const r = 0.9 * tw;
      return node("polygon", {
        key: i,
        points: `${x},${yy - r * 1.6} ${x + r * 0.4},${yy} ${x},${yy + r * 1.6} ${x - r * 0.4},${yy}`,
        fill: "#fff4d6",
        opacity: tw
      });
    });
    return node(Base, {
      ...P,
      mark: node("div", {
        style: {
          position: 'relative',
          width: M,
          height: M,
          flex: 'none',
          opacity: op,
          transform: `translateY(${y}px) rotate(${rot}deg) scale(${sc})`
        }
      }, node("div", {
        style: {
          position: 'absolute',
          left: M * 0.36,
          width: M * 0.28,
          bottom: M * 0.5,
          height: M * 1.6,
          background: `linear-gradient(0deg, ${rgb([240, 192, 80], 0.5)}, ${rgb([240, 192, 80], 0)})`,
          opacity: b,
          transform: `scaleX(${0.3 + 0.7 * b})`,
          transformOrigin: '50% 100%'
        }
      }), node("svg", {
        width: M,
        height: M,
        viewBox: "0 0 32 32",
        fill: "none",
        style: {
          position: 'absolute',
          inset: 0,
          overflow: 'visible'
        }
      }, node("polygon", {
        points: OUTER,
        fill: TILE,
        opacity: 1 - b
      }), edges, node("g", {
        transform: `translate(0 ${-b * 5}) rotate(${60 * b} 16 16)`,
        opacity: 1 - 0.7 * b
      }, node("polygon", {
        points: INNER,
        stroke: CREAM,
        strokeWidth: "1.2",
        fill: b > 0 ? 'none' : PANEL
      })), node("g", {
        transform: `translate(0 ${coreY}) translate(16 16) scale(${1 + 0.7 * b}) translate(-16 -16)`,
        style: {
          filter: `drop-shadow(0 0 ${M * 0.03 * b}px ${GOLD})`
        }
      }, node("polygon", {
        points: CORE,
        fill: rgb(heatRGB(0.78 + 0.2 * b))
      })), stars)),
      word: node(Wordmark, {
        S: S,
        font: font,
        iColor: GOLD,
        iGlow: `0 0 ${font * 0.3 * b}px ${GOLD}`,
        showNext: showNext,
        tag: {
          fill: b,
          fillColor: GOLD,
          chip: {
            background: b > 0.5 ? GOLD : CREAM,
            boxShadow: `0 0 ${font * 0.4 * b}px ${rgb([240, 192, 80], 0.8 * b)}`,
            transform: `scale(${1 + 0.08 * b})`
          }
        }
      })
    });
  }

  // ---------- Puzzle: six wedges scattered and turned; they click into place one per beat
  const WEDGES = OUT_V.map((a, i) => {
    const c = OUT_V[(i + 1) % 6];
    const ext = v => [16 + (v[0] - 16) * 1.4, 16 + (v[1] - 16) * 1.4];
    const cx = (16 + a[0] + c[0]) / 3,
      cy = (16 + a[1] + c[1]) / 3;
    const L = Math.hypot(cx - 16, cy - 16);
    const nx = (cx - 16) / L,
      ny = (cy - 16) / L;
    const dist = 5 + 4 * rnd(i, 1);
    return {
      clip: [[16, 16], ext(a), ext(c)].map(q => q.join(',')).join(' '),
      outline: [[16, 16], a, c].map(q => q.join(',')).join(' '),
      cx,
      cy,
      dx: nx * dist + -ny * (rnd(i, 2) - 0.5) * 6,
      dy: ny * dist + nx * (rnd(i, 2) - 0.5) * 6,
      rot: (rnd(i, 3) < 0.5 ? -1 : 1) * (100 + 140 * rnd(i, 4)),
      solve: 0.1 + 0.08 * [3, 0, 4, 1, 5, 2][i]
    };
  });
  function Puzzle(P) {
    const {
      p,
      S,
      horizontal,
      showNext
    } = P;
    const {
      M,
      font
    } = sizes(S, horizontal);
    const done = since(p, 0.6, 0.2);
    const pieces = WEDGES.map((wd, i) => {
      let s = easeInOutBack(clamp((p - wd.solve) / 0.07));
      if (p >= 0.84) s = 1 - Easing.easeInOutCubic(clamp((p - 0.84 - 0.012 * i) / 0.1));
      const k = 1 - s;
      const snap = since(p, wd.solve + 0.07, 0.05);
      return node("g", {
        key: i,
        transform: `translate(${wd.dx * k} ${wd.dy * k}) rotate(${wd.rot * k} ${wd.cx} ${wd.cy})`,
        opacity: 0.45 + 0.55 * clamp(s * 1.5)
      }, node("g", {
        clipPath: `url(#pz${i})`
      }, node("polygon", {
        points: OUTER,
        stroke: CREAM,
        strokeWidth: "1.6"
      }), node("polygon", {
        points: INNER,
        stroke: CREAM,
        strokeWidth: "1.2"
      }), node("polygon", {
        points: CORE,
        fill: GOLD
      })), node("polygon", {
        points: wd.outline,
        stroke: snap >= 0 ? GOLD : LINE,
        strokeWidth: "0.25",
        opacity: snap >= 0 ? 1 - snap : 0.6 * k,
        strokeLinejoin: "round"
      }));
    });
    const letterStyle = i => ({
      opacity: p >= 0.6 + 0.015 * i && p < 0.86 ? 1 : 0.22
    });
    return node(Base, {
      ...P,
      mark: node("svg", {
        width: M,
        height: M,
        viewBox: "0 0 32 32",
        fill: "none",
        style: {
          display: 'block',
          flex: 'none',
          overflow: 'visible'
        }
      }, node("defs", null, WEDGES.map((wd, i) => node("clipPath", {
        key: i,
        id: `pz${i}`
      }, node("polygon", {
        points: wd.clip
      })))), node("polygon", {
        points: OUTER,
        stroke: LINE,
        strokeWidth: "0.25",
        strokeDasharray: "0.8 0.8",
        opacity: p < 0.6 || p >= 0.86 ? 0.8 : 0
      }), pieces, done >= 0 ? hexRing(1 + 0.5 * easeOut(done), GOLD, 0.5, 0.8 * (1 - done), 'r') : null),
      word: node(Wordmark, {
        S: S,
        font: font,
        iColor: GOLD,
        letterStyle: letterStyle,
        showNext: showNext,
        tag: (() => {
          if (p < 0.62 || p >= 0.94) return {
            slot: true,
            chip: {
              opacity: 0
            }
          };
          if (p < 0.68) {
            const u = easeInOutBack((p - 0.62) / 0.06);
            return {
              slot: true,
              chip: {
                transform: `translateX(${(1 - u) * 1.4 * font}px) rotate(${(1 - u) * 24}deg)`,
                opacity: clamp((p - 0.62) / 0.02)
              }
            };
          }
          if (p < 0.84) return {
            slot: true,
            chip: p < 0.72 ? {
              background: GOLD
            } : {}
          };
          const u = Easing.easeInOutCubic((p - 0.84) / 0.1);
          return {
            slot: true,
            chip: {
              transform: `translateX(${u * 1.4 * font}px) rotate(${u * -24}deg)`,
              opacity: 1 - u
            }
          };
        })()
      })
    });
  }

  // ---------- Achievement: a toast rises, the badge flips in, a shine passes, it slides away
  function Achievement(P) {
    const {
      p,
      S,
      w,
      h,
      horizontal,
      showNext
    } = P;
    const fb = (horizontal ? 120 : 92) * S;
    const B = (horizontal ? 240 : 200) * S;
    let ty = 0,
      op = 1;
    if (p < 0.1) {
      const u = easeOut(p / 0.1);
      ty = (1 - u) * h * 0.6;
      op = u;
    } else if (p >= 0.86) {
      const u = Easing.easeInOutCubic(clamp((p - 0.86) / 0.11));
      ty = u * h * 0.6;
      op = 1 - u;
    }
    const spin = easeOut(clamp((p - 0.06) / 0.26)) * Math.PI * 4;
    const ring = since(p, 0.32, 0.18);
    const shine = since(p, 0.36, 0.14);
    const cardW = Math.min(w * 0.9, B + 48 * S + fb * 4.5 + 120 * S);
    return node(Base, {
      ...P,
      mark: node("div", {
        style: {
          position: 'relative',
          width: cardW,
          display: 'flex',
          alignItems: 'center',
          gap: 48 * S,
          padding: `${36 * S}px ${60 * S}px ${36 * S}px ${36 * S}px`,
          background: PANEL,
          border: `${Math.max(1, 2 * S)}px solid ${LINE}`,
          overflow: 'hidden',
          transform: `translateY(${ty}px)`,
          opacity: op,
          boxShadow: `0 ${30 * S}px ${80 * S}px rgba(0,0,0,0.5)`
        }
      }, node("div", {
        style: {
          position: 'relative',
          width: B,
          height: B,
          flex: 'none',
          background: INK,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center'
        }
      }, node(Logo, {
        M: B * 0.72,
        style: {
          transform: `scaleX(${Math.cos(spin)})`
        }
      }, ring >= 0 ? hexRing(1 + 0.45 * easeOut(ring), GOLD, 0.6, 1 - ring, 'r') : null)), node("div", {
        style: {
          display: 'flex',
          flexDirection: 'column',
          gap: 22 * S,
          minWidth: 0,
          flex: 1
        }
      }, node("div", {
        style: {
          fontFamily: MONO,
          fontSize: 24 * S,
          letterSpacing: '0.24em',
          color: GOLD,
          textTransform: 'uppercase'
        }
      }, "Achievement unlocked"), node(Wordmark, {
        S: S,
        font: fb,
        iColor: GOLD,
        showNext: showNext,
        tag: (() => {
          const pop = since(p, 0.32, 0.1);
          return {
            fill: easeOut((p - 0.12) / 0.2),
            fillColor: GOLD,
            chip: pop >= 0 ? {
              background: GOLD,
              transform: `scale(${1 + 0.3 * Math.sin(Math.PI * pop)})`
            } : {}
          };
        })()
      })), shine >= 0 ? node("div", {
        style: {
          position: 'absolute',
          top: 0,
          bottom: 0,
          width: '22%',
          left: `${-25 + shine * 130}%`,
          background: 'linear-gradient(100deg, rgba(230,225,212,0), rgba(230,225,212,0.12), rgba(230,225,212,0))'
        }
      }) : null),
      word: null
    });
  }

  // ---------- Level up: XP pips charge around the hex; at full it bursts and LEVEL UP pops
  const PIPS = 18;
  function LevelUp(P) {
    const {
      p,
      S,
      horizontal,
      showNext
    } = P;
    const {
      M,
      font
    } = sizes(S, horizontal);
    let count = p < 0.6 ? clamp(Math.floor((p - 0.04) / 0.03) + 1, 0, PIPS) : PIPS;
    if (p < 0.04) count = 0;
    if (p >= 0.86) count = Math.round(PIPS * (1 - clamp((p - 0.86) / 0.1)));
    const big = OUT_V.map(v => [16 + (v[0] - 16) * 1.25, 16 + (v[1] - 16) * 1.25]);
    const pips = [];
    for (let k = 0; k < PIPS; k++) {
      const t = (k + 0.5) / PIPS;
      const [x, y] = along(big, t);
      const e = Math.floor(t * 6);
      const a = big[e],
        c = big[(e + 1) % 6];
      const ang = Math.atan2(c[1] - a[1], c[0] - a[0]) * 180 / Math.PI;
      const pop = since(p, 0.04 + 0.03 * k, 0.04);
      const sc = pop >= 0 && p < 0.6 ? 1 + 0.7 * Math.sin(Math.PI * pop) : 1;
      const on = k < count;
      pips.push(node("rect", {
        key: k,
        x: x - 0.9,
        y: y - 0.45,
        width: "1.8",
        height: "0.9",
        transform: `rotate(${ang} ${x} ${y}) translate(${x} ${y}) scale(${sc}) translate(${-x} ${-y})`,
        fill: on ? GOLD : 'none',
        stroke: on ? GOLD : LINE,
        strokeWidth: "0.2"
      }));
    }
    const burst = since(p, 0.62, 0.22);
    const flash = since(p, 0.62, 0.05) >= 0;
    const rays = burst >= 0 ? [...Array(12)].map((_, i) => {
      const a = i / 12 * Math.PI * 2 + Math.PI / 12;
      const e = easeOut(burst);
      const r0 = 15 + 6 * e,
        r1 = 17 + 12 * e;
      return node("line", {
        key: i,
        x1: 16 + r0 * Math.cos(a),
        y1: 16 + r0 * Math.sin(a),
        x2: 16 + r1 * Math.cos(a),
        y2: 16 + r1 * Math.sin(a),
        stroke: "#fff4d6",
        strokeWidth: "0.5",
        strokeLinecap: "round",
        opacity: 1 - burst
      });
    }) : null;
    const charge = count / PIPS;
    const lu = p >= 0.62 && p < 0.86 ? (p - 0.62) / 0.24 : -1;
    const letterStyle = i => {
      const d = since(p, 0.63 + 0.018 * i, 0.06);
      return {
        transform: `translateY(${d >= 0 ? -Math.sin(Math.PI * d) * 0.14 * font : 0}px)`
      };
    };
    return node(Base, {
      ...P,
      mark: node("div", {
        style: {
          position: 'relative',
          width: M,
          height: M,
          flex: 'none'
        }
      }, node(Logo, {
        M: M,
        coreFill: flash ? '#ffffff' : rgb(heatRGB(0.75 + 0.25 * charge)),
        style: {
          position: 'absolute',
          inset: 0
        },
        before: node("g", {
          style: {
            filter: `drop-shadow(0 0 ${M * 0.02 * charge}px ${GOLD})`
          }
        }, pips)
      }, burst >= 0 ? hexRing(1 + 0.6 * easeOut(burst), GOLD, 0.6, 1 - burst, 'r') : null, rays), lu >= 0 ? node("div", {
        style: {
          position: 'absolute',
          left: -M * 0.5,
          right: -M * 0.5,
          top: -M * 0.2 - lu * M * 0.12,
          textAlign: 'center',
          fontFamily: PIX,
          fontWeight: 700,
          fontSize: 84 * S,
          letterSpacing: '0.08em',
          color: GOLD,
          opacity: lu > 0.7 ? 1 - (lu - 0.7) / 0.3 : 1,
          transform: `scale(${lu < 0.25 ? easeInOutBack(lu / 0.25) : 1})`,
          textShadow: `0 ${5 * S}px 0 ${INK}, 0 0 ${30 * S}px rgba(240,192,80,0.5)`
        }
      }, "LEVEL UP!") : null),
      word: node(Wordmark, {
        S: S,
        font: font,
        iColor: GOLD,
        letterStyle: letterStyle,
        showNext: showNext,
        tag: {
          fill: charge,
          fillColor: GOLD,
          chip: burst >= 0 && burst < 0.4 ? {
            background: '#fff4d6',
            transform: `scale(${1 + 0.25 * Math.sin(Math.PI * burst / 0.4)})`,
            boxShadow: `0 0 ${font * 0.4}px rgba(240,192,80,0.7)`
          } : {}
        }
      })
    });
  }

  // ---------- Fog of war: a scout spirals in, uncovering the map until it finds the mark
  const FOG = (() => {
    const r = 1.15,
      cells = [];
    for (let row = -16; row <= 16; row++) for (let col = -16; col <= 16; col++) {
      const x = Math.sqrt(3) * r * (col + (row & 1 ? 0.5 : 0)),
        y = 1.5 * r * row;
      const d = Math.hypot(x, y);
      if (d > 15.5 || d < 5.6) continue;
      cells.push({
        x,
        y
      });
    }
    const path = t => {
      const rr = 14.5 * (1 - t),
        th = 3.6 + 2.6 * Math.PI * t;
      return [rr * Math.cos(th), rr * Math.sin(th)];
    };
    cells.forEach((c, i) => {
      c.i = i;
      c.pts = [0, 1, 2, 3, 4, 5].map(k => {
        const a = Math.PI / 6 + k * Math.PI / 3;
        return `${(c.x + r * 0.93 * Math.cos(a)).toFixed(2)},${(c.y + r * 0.93 * Math.sin(a)).toFixed(2)}`;
      }).join(' ');
      c.reveal = Infinity;
      for (let s = 0; s <= 240; s++) {
        const t = s / 240;
        const [px, py] = path(t);
        if (Math.hypot(px - c.x, py - c.y) < 4) {
          c.reveal = 0.04 + 0.5 * t;
          break;
        }
      }
      c.fog = 0.84 + 0.12 * rnd(i, 5);
      c.poi = rnd(i, 6) < 0.06 ? rnd(i, 7) < 0.3 ? GOLD : CREAM : null;
    });
    return {
      cells,
      path
    };
  })();
  function Fog(P) {
    const {
      p,
      S,
      horizontal,
      showNext
    } = P;
    const M = (horizontal ? 560 : 640) * S;
    const font = (horizontal ? 140 : 124) * S;
    const t = clamp((p - 0.04) / 0.5);
    const [sx, sy] = FOG.path(t);
    const scoutOn = p > 0.03 && p < 0.56;
    const cells = FOG.cells.map(c => {
      if (p < c.reveal || p >= c.fog + 0.03) return null;
      const op = p >= c.fog ? 1 - (p - c.fog) / 0.03 : clamp((p - c.reveal) / 0.03);
      const near = scoutOn && Math.hypot(sx - c.x, sy - c.y) < 2.6;
      return node("g", {
        key: c.i,
        opacity: op
      }, node("polygon", {
        points: c.pts,
        fill: near ? '#2a3038' : TILE,
        stroke: LINE,
        strokeWidth: "0.08"
      }), c.poi ? node("circle", {
        cx: c.x,
        cy: c.y,
        r: "0.22",
        fill: c.poi
      }) : null);
    });
    const found = since(p, 0.54, 0.1);
    let logoOp = p >= 0.54 ? clamp((p - 0.54) / 0.04) : 0;
    if (p >= 0.9) logoOp = 1 - clamp((p - 0.9) / 0.07);
    const ls = p >= 0.54 && p < 0.62 ? 0.6 + 0.4 * easeInOutBack((p - 0.54) / 0.08) : 1;
    const trail = [];
    for (let k = 1; k <= 10; k++) trail.push(FOG.path(clamp(t - 0.008 * k)).map(v => v.toFixed(2)).join(','));
    const letterStyle = i => ({
      opacity: p >= 0.56 + 0.02 * i && p < 0.9 ? 1 : 0.22
    });
    return node(Base, {
      ...P,
      mark: node("svg", {
        width: M,
        height: M,
        viewBox: "-16 -16 32 32",
        fill: "none",
        style: {
          display: 'block',
          flex: 'none',
          overflow: 'visible'
        }
      }, cells, node("g", {
        opacity: logoOp,
        transform: `scale(${ls})`
      }, node("g", {
        transform: "translate(-5.92 -5.92) scale(0.37)"
      }, node("polygon", {
        points: OUTER,
        stroke: CREAM,
        strokeWidth: "1.6"
      }), node("polygon", {
        points: INNER,
        stroke: CREAM,
        strokeWidth: "1.2"
      }), node("polygon", {
        points: CORE,
        fill: GOLD
      }))), found >= 0 ? node("circle", {
        cx: "0",
        cy: "0",
        r: 5 + 6 * easeOut(found),
        stroke: GOLD,
        strokeWidth: "0.25",
        opacity: 1 - found
      }) : null, scoutOn ? node("g", {
        opacity: p > 0.52 ? 1 - (p - 0.52) / 0.04 : 1
      }, node("polyline", {
        points: `${sx},${sy} ${trail.join(' ')}`,
        stroke: GOLD,
        strokeWidth: "0.18",
        strokeDasharray: "0.35 0.35",
        opacity: "0.6"
      }), node("circle", {
        cx: sx,
        cy: sy,
        r: "1.6",
        fill: GOLD,
        opacity: "0.15"
      }), node("circle", {
        cx: sx,
        cy: sy,
        r: "0.45",
        fill: GOLD
      })) : null),
      word: node(Wordmark, {
        S: S,
        font: font,
        iColor: GOLD,
        letterStyle: letterStyle,
        showNext: showNext,
        tag: (() => {
          const live = p >= 0.58 && p < 0.9;
          const solved = live ? clamp(Math.floor((p - 0.58) / 0.02), 0, 6) : 0;
          const glyphs = '#%&@$*?/<>';
          const step = Math.floor(p * 60);
          const txt = 'ENGINE'.split('').map((ch, i) => i < solved ? ch : glyphs[Math.floor(rnd(step + i * 7, 8) * glyphs.length)]).join('');
          return {
            text: txt,
            ruleScale: live ? easeOut((p - 0.56) / 0.08) : 0.0,
            chip: {
              opacity: live ? 1 : 0.22,
              background: solved === 6 ? CREAM : 'transparent',
              color: solved === 6 ? INK : CREAM,
              boxShadow: inset(S, CREAM)
            }
          };
        })()
      })
    });
  }

  // ---------- Speedrun: a splits timer runs while each part of the mark locks in
  const SPLITS = [['Outer', 0.2, '-0.42'], ['Inner', 0.34, '-0.18'], ['Core', 0.48, '+0.07'], ['Sindri', 0.6, '-0.31']];
  const runTime = p => Math.max(0, (Math.min(p, 0.6) - 0.05) * 11);
  const fmt = t => `0:${t < 10 ? '0' : ''}${t.toFixed(3)}`;
  function Speedrun(P) {
    const {
      p,
      S,
      horizontal,
      showNext
    } = P;
    const M = (horizontal ? 400 : 340) * S;
    const font = (horizontal ? 132 : 100) * S;
    const reset = p >= 0.86;
    const locked = k => p >= SPLITS[k][1] && !reset;
    const lockFlash = k => since(p, SPLITS[k][1], 0.05) >= 0;
    const part = k => locked(k) ? lockFlash(k) ? GOLD : CREAM : LINE;
    const dash = k => locked(k) ? 'none' : '1 0.8';
    const t = reset || p < 0.05 ? 0 : runTime(p);
    const running = p >= 0.05 && p < 0.6;
    const current = SPLITS.findIndex(sp => p < sp[1]);
    return node(Base, {
      ...P,
      mark: node("svg", {
        width: M,
        height: M,
        viewBox: "0 0 32 32",
        fill: "none",
        style: {
          display: 'block',
          flex: 'none',
          overflow: 'visible'
        }
      }, node("polygon", {
        points: OUTER,
        stroke: part(0),
        strokeWidth: "1.6",
        strokeDasharray: dash(0)
      }), node("polygon", {
        points: INNER,
        stroke: part(1),
        strokeWidth: "1.2",
        strokeDasharray: dash(1)
      }), node("polygon", {
        points: CORE,
        fill: locked(2) ? GOLD : 'none',
        stroke: locked(2) ? 'none' : LINE,
        strokeWidth: "0.4"
      }), SPLITS.slice(0, 3).map((sp, k) => {
        const u = since(p, sp[1], 0.12);
        return u >= 0 ? node("g", {
          key: k
        }, hexRing(k === 0 ? 1 + 0.25 * easeOut(u) : k === 1 ? 0.54 + 0.2 * easeOut(u) : 0.22 + 0.2 * easeOut(u), GOLD, 0.4, 1 - u, k)) : null;
      })),
      word: node("div", {
        style: {
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'stretch',
          gap: 28 * S
        }
      }, node(Wordmark, {
        S: S,
        font: font,
        iColor: GOLD,
        showNext: showNext,
        tag: {
          fill: reset ? 0 : clamp((p - 0.05) / 0.55),
          fillColor: GOLD,
          chip: {
            opacity: locked(3) ? 1 : 0.22,
            background: locked(3) && lockFlash(3) ? GOLD : CREAM
          }
        },
        letterStyle: () => ({
          opacity: locked(3) ? 1 : 0.22,
          color: lockFlash(3) && locked(3) ? GOLD : undefined
        })
      }), node("div", {
        style: {
          display: 'flex',
          flexDirection: 'column',
          fontFamily: MONO
        }
      }, node("div", {
        style: {
          fontSize: 64 * S,
          fontWeight: 600,
          color: running || reset || p < 0.05 ? CREAM : GOLD,
          letterSpacing: '0.02em',
          paddingBottom: 14 * S,
          fontVariantNumeric: 'tabular-nums'
        }
      }, fmt(t)), SPLITS.map((sp, k) => {
        const doneRow = p >= sp[1] && !reset;
        return node("div", {
          key: k,
          style: {
            display: 'flex',
            alignItems: 'center',
            gap: 20 * S,
            fontSize: 24 * S,
            padding: `${10 * S}px ${12 * S}px`,
            borderTop: `1px solid ${LINE}`,
            background: k === current && running ? TILE : 'transparent',
            fontVariantNumeric: 'tabular-nums'
          }
        }, node("span", {
          style: {
            flex: 1,
            color: MUTED,
            letterSpacing: '0.14em',
            textTransform: 'uppercase'
          }
        }, sp[0]), node("span", {
          style: {
            color: sp[2][0] === '-' ? GOLD : EMBER,
            opacity: doneRow ? 1 : 0
          }
        }, sp[2]), node("span", {
          style: {
            color: doneRow ? CREAM : MUTED,
            minWidth: 140 * S,
            textAlign: 'right'
          }
        }, doneRow ? fmt(runTime(sp[1])) : '—'));
      })))
    });
  }

  // ---------- Torch: a flickering torch searches a dark stone wall and finds the mark carved into it
  function Torch(P) {
    const {
      p,
      S,
      w,
      h,
      horizontal,
      showNext,
      showProgress
    } = P;
    const {
      M,
      font
    } = sizes(S, horizontal);
    const tau = 2 * Math.PI * p;
    const reach = Math.pow(0.5 + 0.5 * Math.cos(tau), 1.4);
    const tx = w / 2 + w * 0.42 * reach * Math.cos(2.4 + tau);
    const ty = h / 2 + h * 0.3 * reach * Math.sin(2.4 + tau);
    const flick = 1 + 0.05 * Math.sin(2 * Math.PI * 7 * p) + 0.03 * Math.sin(2 * Math.PI * 13 * p + 1) + 0.025 * Math.sin(2 * Math.PI * 29 * p + 2);
    const Rl = Math.min(w, h) * 0.5 * flick;
    const ember = clamp(Math.sin(Math.PI * clamp((p - 0.28) / 0.66)) * 1.4) * flick;
    const bw = 120 * S,
      bh = 56 * S;
    const carved = sw => ({
      stroke: '#07090c',
      strokeWidth: sw
    });
    const carvedMark = node("svg", {
      width: M,
      height: M,
      viewBox: "0 0 32 32",
      fill: "none",
      style: {
        display: 'block',
        flex: 'none',
        overflow: 'visible'
      }
    }, node("g", {
      transform: "translate(0 0.35)"
    }, node("polygon", {
      points: OUTER,
      stroke: "#3d434c",
      strokeWidth: "1.6"
    }), node("polygon", {
      points: INNER,
      stroke: "#3d434c",
      strokeWidth: "1.2"
    }), node("polygon", {
      points: CORE,
      fill: "#3d434c"
    })), node("polygon", {
      points: OUTER,
      ...carved(1.6)
    }), node("polygon", {
      points: INNER,
      ...carved(1.2)
    }), node("polygon", {
      points: CORE,
      fill: "#07090c"
    }));
    const emberMark = node("svg", {
      width: M,
      height: M,
      viewBox: "0 0 32 32",
      fill: "none",
      style: {
        display: 'block',
        flex: 'none',
        overflow: 'visible',
        filter: `drop-shadow(0 0 ${M * 0.035 * ember}px ${EMBER})`
      }
    }, node("polygon", {
      points: CORE,
      fill: rgb(heatRGB(0.45 + 0.4 * ember)),
      opacity: 0.15 + 0.85 * ember
    }));
    const carvedLetters = () => ({
      color: '#0a0d11',
      textShadow: `0 ${3 * S}px 0 #3d434c, 0 ${-2 * S}px 0 #000`
    });
    return node("div", {
      style: {
        position: 'absolute',
        inset: 0,
        background: '#161a20',
        overflow: 'hidden'
      }
    }, node("svg", {
      width: w,
      height: h,
      style: {
        position: 'absolute',
        inset: 0
      }
    }, node("defs", null, node("pattern", {
      id: "bricks",
      width: bw,
      height: bh * 2,
      patternUnits: "userSpaceOnUse"
    }, node("rect", {
      width: bw,
      height: bh * 2,
      fill: "#181c23"
    }), node("rect", {
      x: 2 * S,
      y: 2 * S,
      width: bw - 4 * S,
      height: bh - 4 * S,
      fill: "#1d222a"
    }), node("rect", {
      x: -bw / 2 + 2 * S,
      y: bh + 2 * S,
      width: bw - 4 * S,
      height: bh - 4 * S,
      fill: "#1b2027"
    }), node("rect", {
      x: bw / 2 + 2 * S,
      y: bh + 2 * S,
      width: bw - 4 * S,
      height: bh - 4 * S,
      fill: "#1e232b"
    }))), node("rect", {
      width: w,
      height: h,
      fill: "url(#bricks)"
    })), node(Layout, {
      bare: true,
      S: S,
      horizontal: horizontal,
      showProgress: false,
      mark: carvedMark,
      word: node(Wordmark, {
        S: S,
        font: font,
        iColor: "#0a0d11",
        letterStyle: carvedLetters,
        showNext: showNext,
        tag: {
          ruleColor: '#07090c',
          chip: {
            background: '#0a0d11',
            color: '#3d434c',
            boxShadow: `inset 0 ${-2 * S}px 0 #000, inset 0 ${2 * S}px 0 #2a3038`
          }
        }
      })
    }), node("div", {
      style: {
        position: 'absolute',
        inset: 0,
        background: `radial-gradient(circle ${Rl}px at ${tx}px ${ty}px, rgba(255,160,70,0.16) 0%, rgba(13,17,23,0.35) 45%, rgba(4,5,7,0.97) 100%)`
      }
    }), node(Layout, {
      bare: true,
      S: S,
      horizontal: horizontal,
      showProgress: showProgress,
      progress: node(Progress, {
        p: p,
        S: S,
        w: w,
        color: EMBER,
        label: P.label
      }),
      mark: emberMark,
      word: node(Wordmark, {
        S: S,
        font: font,
        iColor: rgb(heatRGB(0.45 + 0.4 * ember), 0.15 + 0.85 * ember),
        iGlow: `0 0 ${font * 0.3 * ember}px ${EMBER}`,
        letterStyle: i => i === 1 || i === 5 ? {} : {
          opacity: 0
        },
        showNext: showNext,
        tag: {
          ruleOp: 0,
          chip: {
            background: 'transparent',
            color: rgb(heatRGB(0.45 + 0.4 * ember), ember),
            boxShadow: `inset 0 0 0 ${Math.max(1, 2 * S)}px ${rgb(heatRGB(0.45 + 0.4 * ember), 0.7 * ember)}, 0 0 ${font * 0.25 * ember}px ${rgb([212, 84, 30], 0.6 * ember)}`,
            textShadow: `0 0 ${font * 0.12 * ember}px ${EMBER}`
          }
        }
      })
    }));
  }
  const CONCEPTS = {
    forge: Forge,
    blueprint: Blueprint,
    pixel: Pixel,
    cube: Cube,
    arcade: Arcade,
    diorama: Diorama,
    boss: Boss,
    loot: Loot,
    puzzle: Puzzle,
    achievement: Achievement,
    levelup: LevelUp,
    fog: Fog,
    speedrun: Speedrun,
    torch: Torch
  };
  // ---- Which one, and the loop that plays it. ----

  // At random each time, and never the one this browser showed last, so a
  // reload is always something new.
  const names = Object.keys(CONCEPTS);
  let last = null;
  try { last = localStorage.getItem("sindri-loading-concept"); } catch {}
  const choices = names.filter((name) => name !== last);
  const chosen = choices[Math.floor(Math.random() * choices.length)];
  try { localStorage.setItem("sindri-loading-concept", chosen); } catch {}
  const Concept = CONCEPTS[chosen];

  const stage = document.createElement("div");
  stage.className = "motion";
  screen.prepend(stage);
  screen.dataset.motion = chosen;
  // How long one loop takes, for the page's own script: an animation is
  // played through once before the game is revealed, never cut off halfway.
  screen.dataset.motionMs = String(LOOP_MS);

  const opened = performance.now();
  const frame = (now) => {
    if (!screen.isConnected) return;
    const w = stage.clientWidth || innerWidth;
    const h = stage.clientHeight || innerHeight;
    const S = Math.min(w, h) / 1080;
    const p = (((now - opened) % LOOP_MS) + LOOP_MS) % LOOP_MS / LOOP_MS;
    const tree = node(Concept, { p, S, w, h, horizontal: w / h >= 1.25, showNext: true, showProgress: true, label: "Loading.." });
    patch(stage, stage.firstChild, tree, false);
    requestAnimationFrame(frame);
  };
  frame(opened);
})();
