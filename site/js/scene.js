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

  const request = () => {
    if (!queued) {
      queued = true;
      requestAnimationFrame(update);
    }
  };

  addEventListener("scroll", request, { passive: true });
  addEventListener("resize", request);
  request();
})();
