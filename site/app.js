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

  /* Hex field: under every section, cells near the pointer change colour and
     rise a little; a click lifts one higher. Drawn on a canvas, only while the
     section is on screen and something is moving. */
  const secs = [...doc.querySelectorAll(".sec:not(.marquee)")];
  const FR = 28, FGAP = 6, FS = FR + FGAP / SQ;
  const fields = [];
  const pointer = { x: -1e4, y: -1e4, on: false };
  const popAt = [];
  secs.forEach((sec) => {
    const cv = doc.createElement("canvas");
    cv.className = "hexfield";
    cv.setAttribute("aria-hidden", "true");
    sec.insertBefore(cv, sec.firstChild.nextSibling || sec.firstChild);
    fields.push({ sec, cv, ctx: cv.getContext("2d"), cells: new Map(), seen: false, w: 0, h: 0, colors: null });
  });
  const fieldIO = "IntersectionObserver" in window ? new IntersectionObserver((es) => {
    es.forEach((e) => { const f = fields.find((x) => x.sec === e.target); if (f) { f.seen = e.isIntersecting; if (f.seen) kick(); } });
  }) : null;
  fields.forEach((f) => fieldIO && fieldIO.observe(f.sec));
  let raf = 0;
  const kick = () => { if (!raf) raf = requestAnimationFrame(frame); };
  const readColors = (f) => {
    const cs = getComputedStyle(f.sec);
    const g = (n) => cs.getPropertyValue(n).trim();
    f.colors = { accent: g("--color-accent"), ink: g("--color-accent-ink"), s2: g("--color-surface-2"), s3: g("--color-surface-3"), border: g("--color-border"), bg: g("--sec-bg") };
  };
  const size = (f) => {
    const r = f.sec.getBoundingClientRect();
    const dpr = Math.min(devicePixelRatio || 1, 2);
    if (f.w !== r.width || f.h !== r.height) {
      f.w = r.width; f.h = r.height;
      f.cv.width = Math.round(r.width * dpr); f.cv.height = Math.round(r.height * dpr);
      f.dpr = dpr;
    }
    return r;
  };
  const hexAt = (ctx, cx, cy, r) => {
    ctx.beginPath();
    for (let i = 0; i < 6; i++) {
      const a = ((-90 + 60 * i) * Math.PI) / 180;
      const x = cx + r * Math.cos(a), y = cy + r * Math.sin(a);
      i ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
    }
    ctx.closePath();
  };
  const frame = () => {
    raf = 0;
    let busy = false;
    const now = performance.now();
    const motion = !reduce.matches;
    fields.forEach((f) => {
      if (!f.seen) return;
      const r = size(f);
      if (!f.colors) readColors(f);
      const ctx = f.ctx;
      ctx.setTransform(f.dpr, 0, 0, f.dpr, 0, 0);
      ctx.clearRect(0, 0, f.w, f.h);
      const px = pointer.x - r.left, py = pointer.y - r.top;
      const reach = FS * SQ * 2.7;
      const inside = pointer.on && px > -reach && px < f.w + reach && py > -reach && py < f.h + reach;
      const rp = Math.round(py / (FS * 1.5));
      const seen = new Set();
      const draw = [];
      const consider = (q, rr) => {
        const x = FS * SQ * (q + rr / 2), y = FS * 1.5 * rr;
        const d = Math.hypot(x - px, y - py);
        const key = q + "," + rr;
        let lvl = 0;
        if (inside) lvl = d < FS * SQ * 0.55 ? 3 : d < FS * SQ * 1.2 ? 2 : d < FS * SQ * 1.9 ? 1 : d < reach ? 0.5 : 0;
        const st = f.cells.get(key) || { lift: 0 };
        const popping = popAt.find((p) => p.key === key + "|" + f.sec.id + secs.indexOf(f.sec));
        let target = motion ? [0, 1, 3, 6][Math.min(3, Math.ceil(lvl))] * (lvl ? 1 : 0) : 0;
        if (popping) {
          const t = (now - popping.t0) / 700;
          if (t < 1) target += 22 * Math.sin(Math.PI * t) ;
        }
        st.lift += (target - st.lift) * 0.25;
        if (Math.abs(target - st.lift) > 0.2) busy = true;
        if (lvl || st.lift > 0.3) { f.cells.set(key, st); draw.push({ x, y, lvl, lift: st.lift, popping: !!popping }); seen.add(key); }
        else f.cells.delete(key);
      };
      if (inside) {
        for (let rr = rp - 4; rr <= rp + 4; rr++) {
          const qc = Math.round(px / (FS * SQ) - rr / 2);
          for (let q = qc - 4; q <= qc + 4; q++) consider(q, rr);
        }
      }
      f.cells.forEach((st, key) => {
        if (seen.has(key)) return;
        const [q, rr] = key.split(",").map(Number);
        consider(q, rr);
      });
      draw.sort((a, b) => a.y - b.y);
      const c = f.colors;
      draw.forEach((d) => {
        const fill = d.popping || d.lvl >= 3 ? c.accent : d.lvl >= 2 ? c.s3 : c.s2;
        if (d.lift > 0.5) {
          ctx.fillStyle = c.border;
          for (let k = Math.ceil(d.lift); k >= 1; k -= 1) { hexAt(ctx, d.x, d.y - k + d.lift, FR); ctx.fill(); }
        }
        hexAt(ctx, d.x, d.y - d.lift, FR);
        ctx.fillStyle = d.lvl >= 1 || d.lift > 0.5 ? fill : c.bg;
        if (d.lvl < 1 && d.lift <= 0.5) { ctx.strokeStyle = c.s3; ctx.lineWidth = 1; ctx.stroke(); return; }
        ctx.fill();
        ctx.strokeStyle = c.border;
        ctx.lineWidth = 1;
        ctx.stroke();
      });
    });
    for (let i = popAt.length - 1; i >= 0; i--) if (now - popAt[i].t0 > 700) popAt.splice(i, 1); else busy = true;
    if (busy || pointer.on) { if (busy) kick(); }
  };
  addEventListener("pointermove", (e) => { pointer.x = e.clientX; pointer.y = e.clientY; pointer.on = true; kick(); }, { passive: true });
  doc.addEventListener("pointerleave", () => { pointer.on = false; kick(); });
  addEventListener("scroll", () => { if (pointer.on) kick(); }, { passive: true });
  addEventListener("resize", () => { fields.forEach((f) => { f.w = 0; }); kick(); });
  addEventListener("pointerdown", (e) => {
    pointer.x = e.clientX; pointer.y = e.clientY; pointer.on = true;
    fields.forEach((f, i) => {
      const r = f.sec.getBoundingClientRect();
      const px = e.clientX - r.left, py = e.clientY - r.top;
      if (px < 0 || py < 0 || px > r.width || py > r.height) return;
      const rr = Math.round(py / (FS * 1.5));
      const q = Math.round(px / (FS * SQ) - rr / 2);
      popAt.push({ key: q + "," + rr + "|" + f.sec.id + i, t0: performance.now() });
    });
    kick();
  }, { passive: true });
  darkQueryRefresh();
  function darkQueryRefresh() {
    matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => { fields.forEach((f) => { f.colors = null; }); kick(); });
  }

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

  /* Gentle parallax for side cells, by one transform. */
  const sides = [...doc.querySelectorAll(".side")];
  let ticking = false;
  const parallax = () => {
    ticking = false;
    if (reduce.matches) return;
    if (hiveEl && !reduce.matches && innerWidth >= 960) {
      const p = Math.min(1, Math.max(0, scrollY / innerHeight));
      hiveEl.style.transform = `translate3d(0, ${(p * 60).toFixed(1)}px, 0) scale(${(1 - p * 0.12).toFixed(3)})`;
    }
    const rects = sides.map((el) => el.parentElement.getBoundingClientRect());
    sides.forEach((el, i) => {
      const r = rects[i];
      if (r.bottom < 0 || r.top > innerHeight) return;
      const t = r.top + r.height / 2 - innerHeight / 2;
      el.style.transform = `translate3d(0, ${(-t * 0.06).toFixed(1)}px, 0)`;
    });
  };
  const hiveEl = doc.getElementById("hive");
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
  const RINGS = 3;
  const halfW = S * SQ * RINGS + (R * SQ) / 2 + PAD;
  const halfH = S * 1.5 * RINGS + R + PAD;
  const pos = spiral(RINGS).map(([q, r]) => {
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
    if (!visible || hovering || running) { laterAuto(loop, 600); return; }
    const u = unit() || 100;
    const n = Math.min(kind.cells.length, 6);
    for (let i = 0; i < n; i++) {
      later(() => { if (autoplay && !running) { setActive(i); setCenter(kind.cells[i].n); } }, u * 3 * (i + 1));
    }
    later(() => run(0), u * 3 * (n + 1));
    laterAuto(() => {
      if (!autoplay) return;
      const next = cfg.kinds[(cfg.kinds.indexOf(kind) + 1) % cfg.kinds.length];
      renderKind(next);
      setActive(0);
      laterAuto(loop, u * 8);
    }, u * 3 * (n + 1) + u * 26);
  };
  if (autoplay && "IntersectionObserver" in window) {
    new IntersectionObserver((e) => { visible = e[0].isIntersecting; }, { threshold: 0.3 }).observe(stage);
    laterAuto(loop, 1200);
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
