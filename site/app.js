/* SbarHive site behaviour: the interactive hive in the hero, hex edges between
   sections, side cells, reveal on scroll. No dependencies. Geometry matches
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
  const unit = () => parseFloat(getComputedStyle(root).getPropertyValue("--motion-hive-open")) || 0;

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

  /* Gentle parallax for side cells, by one transform. */
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

  /* The hero stage. */
  /* Looping animations pause while off screen. */
  if ("IntersectionObserver" in window) {
    const off = new IntersectionObserver((entries) => {
      entries.forEach((e) => e.target.classList.toggle("off", !e.isIntersecting));
    });
    doc.querySelectorAll(".clips, .marquee").forEach((e) => off.observe(e));
  }

  const stage = doc.getElementById("stage");
  if (!stage || !cfg) { return; }
  const cellsG = doc.getElementById("cells");
  const kindsEl = doc.getElementById("kinds");
  const filesEl = doc.getElementById("files");
  const tray = doc.getElementById("tray");
  const trayName = doc.getElementById("tray-name");
  const trayBar = doc.getElementById("tray-bar");
  const live = doc.getElementById("stage-live");
  const steps = [...stage.querySelectorAll(".stage-steps li")];
  const R = cfg.r, GAP = cfg.gap, PAD = cfg.pad;
  const S = R + GAP / SQ;
  const halfW = S * SQ * 2 + (R * SQ) / 2 + PAD;
  const halfH = S * 1.5 * 2 + R + PAD;
  const pos = spiral(2).map(([q, r]) => {
    let x = S * (SQ * q + (SQ / 2) * r);
    if (cfg.rtl) x = -x;
    return [halfW + x, halfH + S * 1.5 * r];
  });

  let kind = cfg.kinds[0];
  let running = false;
  let autoplay = !reduce.matches;
  let hovering = false;
  let timers = [];
  const later = (fn, ms) => { const t = setTimeout(fn, ms); timers.push(t); return t; };
  const clearTimers = () => { timers.forEach(clearTimeout); timers = []; };
  let autoTimer = 0;
  const laterAuto = (fn, ms) => { clearTimeout(autoTimer); autoTimer = setTimeout(fn, ms); };

  const setStep = (n) => steps.forEach((li, i) => li.classList.toggle("on", i <= n));
  const wrap = (text, max = 9) => {
    const words = text.split(" ");
    const lines = [];
    let cur = "";
    words.forEach((w) => {
      if (cur && (cur + " " + w).length > max) { lines.push(cur); cur = w; }
      else cur = cur ? cur + " " + w : w;
    });
    if (cur) lines.push(cur);
    return lines;
  };
  const setCenter = (text) => {
    const t = cellsG.querySelector(".cell-center text");
    if (!t) return;
    t.textContent = "";
    const lines = wrap(text);
    const [cx, cy] = pos[0];
    t.setAttribute("x", cx);
    lines.forEach((ln, i) => {
      const ts = svg("tspan", { x: cx, y: (cy + (i - (lines.length - 1) / 2) * 13).toFixed(1) });
      ts.textContent = ln;
      t.appendChild(ts);
    });
  };
  const setActive = (idx) => {
    cellsG.querySelectorAll(".cell").forEach((g) => g.classList.toggle("cell-active", Number(g.dataset.idx) === idx));
  };

  const renderFiles = (fresh) => {
    filesEl.textContent = "";
    kind.files.forEach((f) => {
      const li = doc.createElement("li");
      li.innerHTML = '<span class="name"></span><span class="note"></span>';
      li.firstChild.textContent = f;
      li.lastChild.textContent = cfg.ui.original;
      filesEl.appendChild(li);
    });
    if (fresh) {
      const li = doc.createElement("li");
      li.className = "fresh";
      li.innerHTML = '<span class="name"></span><span class="note"></span>';
      li.firstChild.textContent = fresh;
      li.lastChild.textContent = cfg.ui.fresh;
      filesEl.appendChild(li);
    }
  };

  const resetTray = () => {
    tray.classList.remove("on");
    trayBar.classList.remove("run");
  };

  const run = (idx) => {
    if (running) return;
    const cell = kind.cells[idx];
    running = true;
    setActive(idx);
    setStep(2);
    setCenter(cfg.ui.working);
    trayName.textContent = cfg.ui.tray + ": " + cfg.ui.working;
    trayBar.classList.remove("run");
    void trayBar.offsetWidth;
    tray.classList.add("on");
    trayBar.classList.add("run");
    const u = unit();
    later(() => {
      trayName.textContent = cfg.ui.tray + ": " + cfg.ui.doneNote + cell.o;
      renderFiles(cell.o);
      if (!autoplay) live.textContent = cell.n + ": " + cell.o;
      setCenter(kind.selection);
      later(() => { resetTray(); running = false; if (!autoplay) setActive(-1); }, u * 14);
    }, u * 11);
  };

  const renderKind = (k) => {
    kind = k;
    clearTimers();
    running = false;
    resetTray();
    cellsG.textContent = "";
    pos.forEach(([cx, cy], i) => {
      const g = svg("g", { class: i === 0 ? "cell cell-center" : "cell" });
      g.style.setProperty("--i", i);
      g.appendChild(svg("path", { d: hexPath(cx, cy, R) }));
      const t = svg("text", { x: cx.toFixed(1), y: cy.toFixed(1) });
      g.appendChild(t);
      if (i === 0) {
        cellsG.appendChild(g);
        return;
      }
      const idx = i - 1;
      if (idx >= k.cells.length) {
        g.setAttribute("class", "cell cell-empty");
        g.removeChild(t);
      } else {
        const c = k.cells[idx];
        t.textContent = c.c;
        g.dataset.idx = idx;
        g.setAttribute("role", "button");
        g.setAttribute("tabindex", "0");
        g.setAttribute("aria-label", c.n);
        g.addEventListener("pointerenter", () => { if (!running) { setActive(-1); setCenter(c.n); } });
        g.addEventListener("pointerleave", () => { if (!running) setCenter(k.selection); });
        g.addEventListener("focus", () => { if (!running) { setActive(-1); setCenter(c.n); } });
        g.addEventListener("blur", () => { if (!running) setCenter(k.selection); });
        g.addEventListener("click", () => run(idx));
        g.addEventListener("keydown", (e) => {
          if (e.key === "Enter" || e.key === " ") { e.preventDefault(); run(idx); }
        });
      }
      cellsG.appendChild(g);
    });
    setCenter(k.selection);
    renderFiles();
    setStep(1);
    kindsEl.querySelectorAll(".kind").forEach((b) => {
      const on = b.dataset.key === k.key;
      b.setAttribute("aria-checked", on);
      b.tabIndex = on ? 0 : -1;
    });
  };

  cfg.kinds.forEach((k) => {
    const b = doc.createElement("button");
    b.type = "button";
    b.className = "kind";
    b.setAttribute("role", "radio");
    b.dataset.key = k.key;
    b.textContent = k.label;
    b.addEventListener("click", () => renderKind(k));
    kindsEl.appendChild(b);
  });
  kindsEl.addEventListener("keydown", (e) => {
    const dir = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[e.key];
    if (!dir) return;
    const rtl = cfg.rtl && (e.key === "ArrowLeft" || e.key === "ArrowRight") ? -1 : 1;
    const i = cfg.kinds.indexOf(kind);
    const next = cfg.kinds[(i + dir * rtl + cfg.kinds.length) % cfg.kinds.length];
    renderKind(next);
    live.textContent = next.label;
    kindsEl.querySelector(`[data-key="${next.key}"]`).focus();
    e.preventDefault();
  });

  renderKind(kind);
  setActive(0);

  /* Autoplay while visible, until the visitor touches the stage. */
  const stopAuto = () => { autoplay = false; clearTimeout(autoTimer); };
  ["pointerdown", "keydown", "focusin"].forEach((ev) => stage.addEventListener(ev, stopAuto, { once: true }));
  stage.addEventListener("pointerenter", () => { hovering = true; });
  stage.addEventListener("pointerleave", () => { hovering = false; });

  let visible = false;
  const loop = () => {
    if (!autoplay) return;
    if (!visible || hovering || running) { laterAuto(loop, unit() * 10 || 1200); return; }
    const u = unit();
    setActive(0);
    run(0);
    laterAuto(() => {
      if (!autoplay) return;
      const next = cfg.kinds[(cfg.kinds.indexOf(kind) + 1) % cfg.kinds.length];
      renderKind(next);
      setActive(0);
      laterAuto(loop, u * 18);
    }, u * 11 + u * 14 + u * 18);
  };
  if (autoplay && "IntersectionObserver" in window) {
    new IntersectionObserver((e) => { visible = e[0].isIntersecting; }, { threshold: 0.5 }).observe(stage);
    laterAuto(loop, unit() * 25 || 3000);
  }
  reduce.addEventListener("change", () => {
    if (!reduce.matches) return;
    stopAuto();
    sides.forEach((el) => { el.style.transform = ""; });
  });

  /* A visible pause control for every looping animation (WCAG 2.2.2). */
  const toggle = doc.getElementById("motion-toggle");
  if (toggle) {
    toggle.addEventListener("click", () => {
      const paused = root.classList.toggle("paused");
      toggle.setAttribute("aria-pressed", paused);
      toggle.textContent = paused ? toggle.dataset.off : toggle.dataset.on;
      if (paused) stopAuto();
    });
  }
})();
