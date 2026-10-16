// The loading screen's animations: one of the Sindri concepts, chosen at
// random each time the page opens, then -- when the project has a brand -- the
// handoff to it and the brand's own entrance. A classic script inlined in the
// page rather than a file it fetches, so it runs during the parse that shows
// the screen: there is nothing to wait for before the first frame, which is
// the whole of the wait the screen covers.
//
// Four files, one function: this one opens it and holds the renderer,
// `concepts.js` the Sindri concepts, `brand.js` the handoff and entrances, and
// `play.js` chooses, plays and closes it. `page.rs` joins them in that order.
//
// Each scene is a pure function of where it is in time, so the last frame of a
// loop meets the first. The concepts were designed as JSX and are kept as the
// calls JSX compiles to, against `node` below: a tree of plain objects,
// rendered by patching the elements of the frame before, so the page needs no
// framework to play them.
//
// With `prefers-reduced-motion` none of this runs, and the still forge mark
// and the brand's still stage in the page's own markup are the loading screen.
(() => {
  const screen = document.querySelector("#sindri-loading");
  if (!screen || matchMedia("(prefers-reduced-motion: reduce)").matches) return;

  const LOOP_MS = 4000;

  // How much of the wait is over, 0 to 1, as the page's own script measures
  // it; `null` until anything has been measured.
  const loadFraction = () => {
    const value = screen.dataset.progress;
    return value === undefined ? null : Number(value);
  };
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

