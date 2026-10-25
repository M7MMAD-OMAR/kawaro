/* SbarHive site: the pinned scroll scene. One rAF loop, fed by scroll (and resize)
   only. It reads where the section is, turns that into a progress p in [0, 1],
   writes --p on the section (the CSS turns p into transform and opacity) and sets
   data-state, data-selected and data-picked. Thresholds come from the site tokens
   (--scene-*), so CSS and JS cannot drift. With reduced motion the scene is static
   and this loop does nothing. */
(() => {
  const scene = document.getElementById("scene");
  if (!scene) return;
  const reduce = matchMedia("(prefers-reduced-motion: reduce)");
  const css = getComputedStyle(scene);
  const token = (name) => parseFloat(css.getPropertyValue(name));
  const at = {
    select: token("--scene-select"),
    open: token("--scene-open"),
    pick: token("--scene-pick"),
    result: token("--scene-result"),
  };
  const original = scene.querySelector(".scene-original");
  const picked = scene.querySelector(".scene-cell[data-pick]");
  const result = scene.querySelector(".scene-result");
  const ghosts = [...scene.querySelectorAll(".scene-ghost")];
  const cells = [...scene.querySelectorAll(".scene-cell")];
  let last = "";
  let queued = false;

  const stateOf = (p) =>
    p >= at.result ? "result" : p >= at.pick ? "pick" : p >= at.open ? "open" : p >= at.select ? "select" : "folder";

  const update = () => {
    queued = false;
    if (reduce.matches) return;
    const span = scene.offsetHeight - innerHeight;
    const p = span > 0 ? Math.min(1, Math.max(0, -scene.getBoundingClientRect().top / span)) : 0;
    const key = p.toFixed(4);
    if (key === last) return;
    last = key;
    scene.style.setProperty("--p", key);
    const state = stateOf(p);
    if (scene.dataset.state !== state) {
      scene.dataset.state = state;
      const past = state !== "folder";
      if (original) original.toggleAttribute("data-selected", past);
      if (picked) picked.toggleAttribute("data-picked", state === "pick" || state === "result");
    }
  };

  /* Where each cell starts (the original file tile) and where the result starts (the
     picked cell), as pixel distances from the real rects. Ghost cells sit at the cell
     slots and carry no transform, so they give the resting places. Run on load and
     resize only, never on scroll. */
  const centre = (el) => {
    const r = el.getBoundingClientRect();
    return [r.left + r.width / 2, r.top + r.height / 2];
  };
  const measure = () => {
    if (!original || !ghosts.length) return;
    const [ox, oy] = centre(original);
    cells.forEach((cell, i) => {
      if (!ghosts[i]) return;
      const [gx, gy] = centre(ghosts[i]);
      cell.style.setProperty("--sx", (ox - gx).toFixed(1) + "px");
      cell.style.setProperty("--sy", (oy - gy).toFixed(1) + "px");
    });
    if (result && ghosts[1]) {
      result.style.transform = "none";
      const [rx, ry] = centre(result);
      result.style.removeProperty("transform");
      const [px, py] = centre(ghosts[1]);
      result.style.setProperty("--rx", (px - rx).toFixed(1) + "px");
      result.style.setProperty("--ry", (py - ry).toFixed(1) + "px");
    }
  };

  const request = () => {
    if (!queued) {
      queued = true;
      requestAnimationFrame(update);
    }
  };

  addEventListener("scroll", request, { passive: true });
  addEventListener("resize", () => { measure(); request(); });
  addEventListener("load", measure);
  if (document.fonts && document.fonts.ready) document.fonts.ready.then(measure);
  measure();
  request();
})();
