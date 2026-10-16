
  // ---- Which concept, and the loop that plays it. ----

  // At random each time, and never the one this browser showed last, so a
  // reload is always something new.
  const names = Object.keys(CONCEPTS);
  let last = null;
  try { last = localStorage.getItem("sindri-loading-concept"); } catch {}
  const choices = names.filter((name) => name !== last);
  const chosen = choices[Math.floor(Math.random() * choices.length)];
  try { localStorage.setItem("sindri-loading-concept", chosen); } catch {}
  const Concept = CONCEPTS[chosen];

  const brandStage = screen.querySelector(".stage.brand");
  const brand = brandStage ? readBrand(brandStage) : null;

  const stage = document.createElement("div");
  stage.className = "motion";
  screen.prepend(stage);
  screen.dataset.motion = chosen;
  // What the page's own script times the screen by: one whole loop before
  // anything replaces it, and for a brand, the handoff before its time starts
  // and the entrance as the least that time can be.
  screen.dataset.motionMs = String(LOOP_MS);
  if (brand) {
    screen.dataset.brandEntrance = brand.entrance;
    screen.dataset.brandLeadMs = String(HANDOFF_MS);
    screen.dataset.brandEntranceMs = String(ENTRANCE_MS + HOLD_MS);
  }

  const opened = performance.now();
  let brandAt = null;
  const frame = (now) => {
    if (!screen.isConnected) return;
    const w = stage.clientWidth || innerWidth;
    const h = stage.clientHeight || innerHeight;
    const S = Math.min(w, h) / 1080;
    const horizontal = w / h >= 1.25;
    const p = (((now - opened) % LOOP_MS) + LOOP_MS) % LOOP_MS / LOOP_MS;
    if (brand && brandAt === null && screen.dataset.stage === "brand") brandAt = now;
    const tree = brandAt === null
      ? node(Concept, { p, S, w, h, horizontal, showNext: true, showProgress: true, label: "Loading.." })
      : node(BrandScreen, { t: now - brandAt, p, S, w, h, horizontal, brand, Concept });
    patch(stage, stage.firstChild, tree, false);
    requestAnimationFrame(frame);
  };
  frame(opened);
})();
