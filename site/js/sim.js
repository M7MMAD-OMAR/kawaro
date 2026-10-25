/* SbarHive site: the try-it demo (the old keyboard operable drop demo behind the
   control after the scene). A sample file dropped on the folder, or chosen with
   Enter or Space, opens a hive with the cells of its kind from the build; a
   picked cell adds the result next to the original. Nothing leaves the page.
   Geometry matches spikes/hive/src/geometry.rs and site/build.py: pointy-top
   cells in axial coordinates, centre first then each ring, x mirrored for right
   to left. */
(() => {
  const doc = document;
  const SQ = Math.sqrt(3);
  const cfgNode = doc.getElementById("hive-config");
  const cfg = cfgNode ? JSON.parse(cfgNode.textContent) : null;
  const DIRS = [[1, 0], [1, -1], [0, -1], [-1, 0], [-1, 1], [0, 1]];
  const spiral = (rings) => {
    const out = [[0, 0]];
    for (let k = 1; k <= rings; k++) {
      let q = -k, r = k;
      for (const [dq, dr] of DIRS) {
        for (let i = 0; i < k; i++) { out.push([q, r]); q += dq; r += dr; }
      }
    }
    return out;
  };
  // sim:begin
  /* Try it. A sample file dropped on the folder, or chosen with Enter or Space,
     opens a hive with the cells of its kind from the build; a picked cell adds
     the result next to the original. Nothing leaves the page. */
  const sim = doc.getElementById("sim");
  if (sim && cfg) {
    const stageEl = sim.querySelector(".sim-stage");
    const zone = doc.getElementById("sim-zone");
    const hive = doc.getElementById("sim-hive");
    const list = doc.getElementById("sim-files");
    const live = doc.getElementById("stage-live");
    const samples = [...sim.querySelectorAll(".sim-file")];
    const byKey = (k) => cfg.kinds.find((x) => x.key === k);
    const unitX = (cfg.r + cfg.gap / SQ) / cfg.r;
    const slots = spiral(2);
    let cur = null, shown = false, results = [], dragKey = "", quiet = false, centre = null;

    const place = (el, i) => {
      const [q, r] = slots[i];
      el.style.setProperty("--x", ((cfg.rtl ? -1 : 1) * unitX * (q + r / 2)).toFixed(4));
      el.style.setProperty("--y", (0.75 * unitX * r).toFixed(4));
    };
    const row = (name, note, mark, cls) => {
      const li = doc.createElement("li");
      li.className = cls;
      li.setAttribute(mark, "");
      const n = doc.createElement("span");
      n.className = "name";
      n.textContent = name;
      li.appendChild(n);
      if (note) {
        const t = doc.createElement("span");
        t.className = "note";
        t.textContent = note;
        li.appendChild(t);
      }
      return li;
    };
    const renderFolder = () => {
      list.textContent = "";
      if (cur) {
        cur.files.forEach((f) => list.appendChild(row(f, cfg.ui.original, "data-original", "")));
        results.forEach((c, i) => {
          const last = i === results.length - 1;
          list.appendChild(row(c.o, last ? cfg.ui.fresh : "", "data-result", last ? "fresh" : ""));
        });
      }
      zone.classList.toggle("full", list.children.length > 0);
    };
    const pick = (i, btn) => {
      const c = cur.cells[i];
      if (!results.includes(c)) results.push(c);
      btn.classList.add("done");
      renderFolder();
      live.textContent = cfg.ui.done + ": " + c.o;
    };
    const open = (key, focusFirst) => {
      const k = byKey(key);
      if (!k) return;
      cur = k;
      shown = true;
      results = [];
      samples.forEach((b) => b.classList.toggle("on", b.dataset.kind === key));
      hive.textContent = "";
      hive.dataset.rings = k.cells.length > 6 ? "2" : "1";
      centre = doc.createElement("div");
      centre.id = "sim-centre";
      centre.className = "sim-centre";
      centre.textContent = k.selection;
      place(centre, 0);
      hive.appendChild(centre);
      k.cells.forEach((c, i) => {
        const b = doc.createElement("button");
        b.type = "button";
        b.className = "sim-cell";
        b.dataset.cell = c.id;
        b.setAttribute("aria-label", c.n);
        b.style.setProperty("--i", i + 1);
        place(b, i + 1);
        b.innerHTML = '<span class="ring"></span><span class="hx"></span>';
        b.lastChild.textContent = c.c;
        const name = () => { centre.textContent = c.n; };
        const back = () => { centre.textContent = k.selection; };
        b.addEventListener("pointerenter", name);
        b.addEventListener("pointerleave", back);
        b.addEventListener("focus", () => { if (!quiet) name(); });
        b.addEventListener("blur", back);
        b.addEventListener("click", () => pick(i, b));
        hive.appendChild(b);
      });
      hive.hidden = false;
      stageEl.classList.add("open");
      renderFolder();
      live.textContent = k.selection + ": " + cfg.ui.actions.replace("{n}", k.cells.length);
      if (focusFirst) {
        quiet = true;
        hive.querySelector(".sim-cell").focus();
        quiet = false;
      }
    };
    const close = () => {
      if (!shown) return;
      shown = false;
      hive.hidden = true;
      hive.textContent = "";
      stageEl.classList.remove("open");
      const s = samples.find((b) => b.dataset.kind === cur.key);
      if (s) s.focus();
    };

    samples.forEach((s) => {
      /* A keyboard or screen reader click has detail 0: move into the hive then. */
      s.addEventListener("click", (e) => open(s.dataset.kind, e.detail === 0));
      s.addEventListener("dragstart", (e) => {
        dragKey = s.dataset.kind;
        e.dataTransfer.setData("text/plain", dragKey);
        e.dataTransfer.effectAllowed = "copy";
        zone.classList.add("ready");
      });
      s.addEventListener("dragend", () => zone.classList.remove("ready", "over"));
    });
    zone.addEventListener("dragover", (e) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = "copy";
      zone.classList.add("over");
    });
    zone.addEventListener("dragleave", () => zone.classList.remove("over"));
    zone.addEventListener("drop", (e) => {
      e.preventDefault();
      zone.classList.remove("over", "ready");
      const key = e.dataTransfer.getData("text/plain") || dragKey;
      if (byKey(key)) open(key, false);
    });
    sim.addEventListener("keydown", (e) => {
      if (e.key === "Escape" && shown) { e.preventDefault(); close(); }
    });
  }
  // sim:end
})();
