/* SbarHive site behaviour: hex edges between sections, side cells, reveal on
   scroll, the download button for the visitor's system, docs search and the
   try-it demo. No dependencies. Geometry matches
   spikes/hive/src/geometry.rs and site/build.py: pointy-top cells in axial
   coordinates, spacing r + gap / sqrt3, centre first then each ring, x mirrored
   for right to left. Everything works without JS except the animation. */
(() => {
  const doc = document;
  const root = doc.documentElement;
  const NS = "http://www.w3.org/2000/svg";
  const SQ = Math.sqrt(3);
  const reduce = matchMedia("(prefers-reduced-motion: reduce)");
  const cfgNode = doc.getElementById("hive-config");
  const cfg = cfgNode ? JSON.parse(cfgNode.textContent) : null;

  const svg = (name, attrs = {}) => {
    const el = doc.createElementNS(NS, name);
    for (const k in attrs) el.setAttribute(k, attrs[k]);
    return el;
  };
  const hexPath = (cx, cy, r) => {
    const pts = [-90, -30, 30, 90, 150, 210].map((a) => {
      const t = (a * Math.PI) / 180;
      return `${(cx + r * Math.cos(t)).toFixed(1)},${(cy + r * Math.sin(t)).toFixed(1)}`;
    });
    return "M" + pts.join("L") + "Z";
  };
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

  /* Hex edges. A full row of cells with a sparse row above it. */
  const EDGE_R = 24, EDGE_W = SQ * EDGE_R, EDGE_H = 3.5 * EDGE_R, EDGE_COLS = 64;
  doc.querySelectorAll(".hexedge").forEach((el) => {
    const s = svg("svg", {
      viewBox: `0 0 ${(EDGE_COLS * EDGE_W).toFixed(0)} ${EDGE_H}`,
      preserveAspectRatio: "xMidYMax slice", focusable: "false",
    });
    s.appendChild(svg("rect", { x: 0, y: 2.5 * EDGE_R, width: EDGE_COLS * EDGE_W, height: EDGE_R }));
    for (let i = 0; i <= EDGE_COLS; i++) {
      const d = Math.abs(i - EDGE_COLS / 2);
      const c = svg("path", { class: "c", d: hexPath(i * EDGE_W, 2.5 * EDGE_R, EDGE_R + 0.5) });
      c.style.setProperty("--d", d);
      s.appendChild(c);
      if ((i * 7 + 3) % 5 < 2) {
        const top = svg("path", {
          class: "c" + ((i * 5) % 11 === 0 ? " hot" : ""),
          d: hexPath(i * EDGE_W + EDGE_W / 2, EDGE_R, EDGE_R + 0.5),
        });
        top.style.setProperty("--d", d + 2);
        s.appendChild(top);
      }
    }
    el.appendChild(s);
  });

  /* Side cells. */
  doc.querySelectorAll(".side").forEach((el, n) => {
    const r = 34, gap = 6, s = r + gap / SQ;
    const cells = spiral(2);
    const s2 = svg("svg", { viewBox: "-130 -130 260 260", focusable: "false" });
    cells.forEach(([q, rr], i) => {
      if ((i + n) % 4 === 3) return;
      const x = s * (SQ * q + (SQ / 2) * rr), y = s * 1.5 * rr;
      const c = svg("path", { class: "c" + ((i + n) % 2 ? " b" : "") + (i === 4 + (n % 3) ? " hot" : ""), d: hexPath(x, y, r) });
      c.style.setProperty("--i", i);
      s2.appendChild(c);
    });
    el.appendChild(s2);
  });

  /* Highlight the download card that matches the visitor's system. */
  const ua = (navigator.userAgent || "").toLowerCase();
  const os = ua.includes("windows") ? "win" : ua.includes("linux") && !ua.includes("android") ? "linux" : "";
  if (os) doc.querySelectorAll(`.dl-card[data-card="${os}"]`).forEach((c) => c.classList.add("rec"));


  /* Docs search with Pagefind. The index loads only when the box is used. */
  const form = doc.querySelector("form.search");
  if (form) {
    const input = form.querySelector("input");
    const list = form.querySelector(".results");
    let pf = null;
    let seq = 0;
    const load = () => pf || (pf = import(new URL(form.dataset.js, location.href).href).then((m) => (m.init && m.init(), m)));
    const run = async () => {
      const q = input.value.trim();
      const my = ++seq;
      list.textContent = "";
      if (!q) return;
      try {
        const api = await load();
        const res = await api.search(q);
        const top = await Promise.all(res.results.slice(0, 6).map((r) => r.data()));
        if (my !== seq) return;
        if (!top.length) {
          const li = doc.createElement("li");
          li.textContent = form.dataset.none;
          list.appendChild(li);
          return;
        }
        top.forEach((d) => {
          const li = doc.createElement("li");
          const a = doc.createElement("a");
          a.href = d.url;
          a.textContent = (d.meta && d.meta.title) || d.url;
          const p = doc.createElement("p");
          p.innerHTML = d.excerpt;
          li.append(a, p);
          list.appendChild(li);
        });
      } catch (e) {
        list.textContent = "";
      }
    };
    input.addEventListener("input", run);
    form.addEventListener("submit", (e) => e.preventDefault());
  }

  /* Reveal on scroll. */
  const revealers = [...doc.querySelectorAll("[data-reveal], .hexedge, .side, .journey-wrap")];
  doc.querySelectorAll(".grid, .ticks, .steps").forEach((g) => {
    [...g.children].forEach((c, i) => c.style.setProperty("--n", i));
  });
  if ("IntersectionObserver" in window) {
    const io = new IntersectionObserver((entries) => {
      entries.forEach((e) => {
        if (e.isIntersecting) { e.target.classList.add("in"); io.unobserve(e.target); }
      });
    }, { rootMargin: "0px 0px -8% 0px" });
    revealers.forEach((e) => io.observe(e));
  } else {
    revealers.forEach((e) => e.classList.add("in"));
  }

  /* Gentle parallax for side cells, by one transform, driven by scrolling only. */
  const sides = [...doc.querySelectorAll(".side")];
  let ticking = false;
  const parallax = () => {
    ticking = false;
    if (reduce.matches) return;
    const rects = sides.map((el) => el.parentElement.getBoundingClientRect());
    sides.forEach((el, i) => {
      const r = rects[i];
      if (r.bottom < 0 || r.top > innerHeight) return;
      const t = r.top + r.height / 2 - innerHeight / 2;
      el.style.transform = `translate3d(0, ${(-t * 0.06).toFixed(1)}px, 0)`;
    });
  };
  addEventListener("scroll", () => { if (!ticking) { ticking = true; requestAnimationFrame(parallax); } }, { passive: true });
  parallax();
  reduce.addEventListener("change", () => {
    if (reduce.matches) sides.forEach((el) => { el.style.transform = ""; });
  });

  /* Looping animations pause while off screen. */
  if ("IntersectionObserver" in window) {
    const off = new IntersectionObserver((entries) => {
      entries.forEach((e) => e.target.classList.toggle("off", !e.isIntersecting));
    });
    doc.querySelectorAll(".marquee, .tile").forEach((e) => off.observe(e));
  }

  /* A visible pause control for every looping animation (WCAG 2.2.2). */
  const toggle = doc.getElementById("motion-toggle");
  if (toggle) {
    toggle.addEventListener("click", () => {
      const paused = root.classList.toggle("paused");
      toggle.setAttribute("aria-pressed", paused);
      toggle.textContent = paused ? toggle.dataset.off : toggle.dataset.on;
    });
  }

  /* The hero download button follows the visitor's system once the release is
     published (data-rel): a user agent with Windows gets the MSI, Linux gets
     the DEB. Without JS, or on another system, it opens the downloads page. */
  const prim = doc.querySelector("a[data-os-default]");
  if (prim && prim.dataset.rel && cfg) {
    const pick = ua.includes("windows") ? ["msi", "win"] : ua.includes("linux") && !ua.includes("android") ? ["deb", "linux"] : null;
    const src = pick && doc.querySelector(`a[data-dl="${pick[0]}"]`);
    if (src) {
      prim.href = src.getAttribute("href");
      prim.textContent = cfg.ui.rel.replace("{os}", cfg.ui[pick[1]]);
    }
  }
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
