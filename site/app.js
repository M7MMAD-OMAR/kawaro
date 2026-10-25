/* SbarHive site behaviour: the download card for the visitor's system, the hero
   download button, and the docs search. No dependencies. Everything works without
   JS. The home page motion is CSS only (S-064, S-065). */
(() => {
  const doc = document;
  const cfgNode = doc.getElementById("hive-config");
  const cfg = cfgNode ? JSON.parse(cfgNode.textContent) : null;

  /* Highlight the download card that matches the visitor's system. */
  const ua = (navigator.userAgent || "").toLowerCase();
  const os = ua.includes("windows") ? "win" : ua.includes("linux") && !ua.includes("android") ? "linux" : "";
  if (os) doc.querySelectorAll(`.download-slab[data-card="${os}"]`).forEach((c) => c.classList.add("rec"));

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
})();
