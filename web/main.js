import init, * as uc from "./pkg/undercroft_wasm.js";
import { TILE, loadImage, paintTiles, toast } from "./shared.js";

const STYLE_ICONS = { dungeon: "2,6", caves: "26,8", overworld: "0,1" };
const NUMBER_LINE = /^(\s*)([A-Za-z_]+)(\s*=\s*)(-?\d+(?:\.\d+)?)(\s*(?:#.*)?)$/;
const SEED_WORDS = ["owl", "moss", "ember", "crypt", "lantern", "hollow", "rune", "ash", "vault", "fern", "tide", "bone", "wick", "gloam"];

const $ = (id) => document.getElementById(id);
const el = {
  viewport: $("viewport"), world: $("world"), canvas: $("canvas"), ascii: $("ascii"),
  topbar: document.querySelector(".topbar"), hud: document.querySelector(".hud"),
  styles: $("styles"), seed: $("seed"), daily: $("daily"), panel: $("panel"),
  panelToggle: $("panel-toggle"), sliders: $("sliders"), recipe: $("recipe"),
  dialog: $("recipe-dialog"), stats: $("stats"), toast: $("toast"), error: $("error"),
  version: $("version"),
};
const ctx = el.canvas.getContext("2d");
const buffer = document.createElement("canvas"); // full latest image, for export and the reveal
const bufferCtx = buffer.getContext("2d");
const narrow = matchMedia("(max-width: 900px)");
const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");

const state = { styles: [], style: null, current: null, daily: null, view: "tiles", sheet: null };
const cam = { x: 0, y: 0, scale: 1, fitted: true };

await init();
state.styles = JSON.parse(uc.styles());
state.sheet = await loadImage("pkg/tiles.png").catch(() => null);
el.version.textContent = uc.genVersion();

function regenerate({ animate = false, refit = false } = {}) {
  const started = performance.now();
  let next;
  try {
    next = state.daily
      ? uc.generateDaily(el.recipe.value, state.daily)
      : uc.generate(el.recipe.value, el.seed.value);
  } catch (e) {
    showError(String(e?.message ?? e));
    return;
  }
  const ms = performance.now() - started;
  showError(null);

  const prev = state.current;
  const resized = !prev || prev.width !== next.width || prev.height !== next.height;
  prev?.free();
  state.current = next;

  draw(next, animate);
  if (state.view === "ascii") renderAscii();
  if (refit || (resized && cam.fitted)) fit(!resized);
  renderStats(ms);
  writeHash();
}

let revealFrame = 0;

/** With `animate`, new tiles dissolve in over the old ones. */
function draw(gen, animate) {
  const pixels = gen.pixels();
  const image = new ImageData(new Uint8ClampedArray(pixels.buffer, pixels.byteOffset, pixels.length), gen.width, gen.height);
  buffer.width = gen.width;
  buffer.height = gen.height;
  bufferCtx.putImageData(image, 0, 0);

  cancelAnimationFrame(revealFrame);
  if (el.canvas.width !== gen.width || el.canvas.height !== gen.height) {
    el.canvas.width = gen.width;
    el.canvas.height = gen.height;
  }
  if (!animate || reducedMotion.matches) {
    ctx.putImageData(image, 0, 0);
    return;
  }

  const cols = gen.width / TILE;
  const order = shuffle(Uint32Array.from({ length: cols * (gen.height / TILE) }, (_, i) => i));
  const perFrame = Math.ceil(order.length / 16);
  let i = 0;
  const step = () => {
    for (const end = Math.min(i + perFrame, order.length); i < end; i++) {
      const x = (order[i] % cols) * TILE;
      const y = Math.floor(order[i] / cols) * TILE;
      ctx.drawImage(buffer, x, y, TILE, TILE, x, y, TILE, TILE);
    }
    if (i < order.length) revealFrame = requestAnimationFrame(step);
  };
  step();
}

function renderAscii() {
  const world = JSON.parse(state.current.json());
  const { width, height, cells } = world.terrain;
  const frag = document.createDocumentFragment();
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width;) {
      const tag = world.tags[cells[y * width + x]];
      let run = "";
      while (x < width && world.tags[cells[y * width + x]] === tag) {
        run += tag.glyph;
        x++;
      }
      const span = document.createElement("span");
      span.textContent = run;
      if (tag.color) span.style.color = `rgb(${tag.color})`;
      frag.append(span);
    }
    frag.append("\n");
  }
  el.ascii.replaceChildren(frag);
}

function renderStats(ms) {
  const g = state.current;
  const parts = [
    [`${g.width / TILE}×${g.height / TILE}`, "b"],
    [`${ms < 10 ? ms.toFixed(1) : Math.round(ms)} ms`],
  ];
  if (g.attempt) parts.push([`${g.attempt} rerolled`, "warn"]);
  parts.push([`#${BigInt(g.seed).toString(16).slice(0, 8)}`]);
  el.stats.replaceChildren(...parts.flatMap(([text, kind], i) => {
    const node = kind === "b" ? document.createElement("b") : document.createElement("span");
    node.textContent = text;
    if (kind === "warn") node.className = "warn";
    return i ? [document.createTextNode("  ·  "), node] : [node];
  }));
}

function showError(message) {
  el.error.hidden = !message;
  el.error.textContent = message ?? "";
}

function applyCam(smooth = false) {
  el.world.classList.toggle("smooth", smooth);
  el.world.style.transform = `translate(${cam.x}px, ${cam.y}px) scale(${cam.scale})`;
}

/** The part of the screen not covered by floating chrome. */
function openArea() {
  const top = el.topbar.getBoundingClientRect().bottom + 16;
  let bottom = el.hud.getBoundingClientRect().top - 16;
  let right = innerWidth - 16;
  if (narrow.matches) bottom = Math.min(bottom, el.panel.getBoundingClientRect().top - 12);
  else if (!el.panel.hasAttribute("data-collapsed")) right = el.panel.getBoundingClientRect().left - 16;
  return { x: 16, y: top, w: right - 16, h: Math.max(bottom - top, 80) };
}

function fit(smooth = true) {
  const area = openArea();
  const w = el.world.offsetWidth;
  const h = el.world.offsetHeight;
  if (!w || !h) return;
  const s = Math.min(area.w / w, area.h / h);
  cam.scale = s >= 1 ? Math.floor(s) : s; // whole-number scales keep pixels crisp
  cam.x = Math.round(area.x + (area.w - w * cam.scale) / 2);
  cam.y = Math.round(area.y + (area.h - h * cam.scale) / 2);
  cam.fitted = true;
  applyCam(smooth);
}

function zoomAt(factor, cx, cy, smooth = false) {
  const scale = Math.min(Math.max(cam.scale * factor, 0.1), 16);
  cam.x = cx - (cx - cam.x) * (scale / cam.scale);
  cam.y = cy - (cy - cam.y) * (scale / cam.scale);
  cam.scale = scale;
  cam.fitted = false;
  applyCam(smooth);
}

function zoomCenter(factor) {
  const a = openArea();
  zoomAt(factor, a.x + a.w / 2, a.y + a.h / 2, true);
}

const pointers = new Map();

el.viewport.addEventListener("pointerdown", (e) => {
  el.viewport.setPointerCapture(e.pointerId);
  pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
  el.viewport.classList.add("dragging");
});

el.viewport.addEventListener("pointermove", (e) => {
  const prev = pointers.get(e.pointerId);
  if (!prev) return;
  const now = { x: e.clientX, y: e.clientY };
  if (pointers.size === 1) {
    cam.x += now.x - prev.x;
    cam.y += now.y - prev.y;
    cam.fitted = false;
    applyCam();
  } else if (pointers.size === 2) {
    const other = [...pointers].find(([id]) => id !== e.pointerId)[1];
    const before = Math.hypot(prev.x - other.x, prev.y - other.y);
    const after = Math.hypot(now.x - other.x, now.y - other.y);
    if (before > 0) zoomAt(after / before, (now.x + other.x) / 2, (now.y + other.y) / 2);
  }
  pointers.set(e.pointerId, now);
});

for (const type of ["pointerup", "pointercancel"]) {
  el.viewport.addEventListener(type, (e) => {
    pointers.delete(e.pointerId);
    if (!pointers.size) el.viewport.classList.remove("dragging");
  });
}

el.viewport.addEventListener("wheel", (e) => {
  e.preventDefault();
  const speed = e.ctrlKey ? 0.01 : 0.0015; // ctrlKey = trackpad pinch
  zoomAt(Math.exp(-e.deltaY * speed), e.clientX, e.clientY);
}, { passive: false });

el.viewport.addEventListener("dblclick", () => fit());
addEventListener("resize", () => { if (cam.fitted) fit(false); });
document.fonts?.ready.then(() => { if (cam.fitted) fit(false); });

$("zoom-in").addEventListener("click", () => zoomCenter(1.5));
$("zoom-out").addEventListener("click", () => zoomCenter(1 / 1.5));
$("zoom-fit").addEventListener("click", () => fit());

function buildStyleTabs() {
  el.styles.replaceChildren(...state.styles.map((s, i) => {
    const button = document.createElement("button");
    button.type = "button";
    button.dataset.style = s.name;
    button.title = `${s.description ?? s.name} (${i + 1})`;
    const icon = document.createElement("canvas");
    Object.assign(icon, { className: "tile", width: TILE, height: TILE });
    icon.dataset.tile = STYLE_ICONS[s.name] ?? "10,17";
    const label = document.createElement("span");
    label.textContent = s.name;
    button.append(icon, label);
    button.addEventListener("click", () => selectStyle(s.name));
    return button;
  }));
}

function selectStyle(name, { keepRecipe = false } = {}) {
  state.style = name;
  for (const b of el.styles.children) b.setAttribute("aria-pressed", b.dataset.style === name);
  paintIcons();
  if (!keepRecipe) el.recipe.value = styleSource(name);
  buildSliders();
  regenerate({ animate: true, refit: true });
}

function setDaily(date) {
  state.daily = date;
  el.daily.setAttribute("aria-pressed", Boolean(date));
  paintIcons();
}

function randomize() {
  setDaily(null);
  el.seed.value = randomSeed();
  regenerate({ animate: true });
}

function goDaily() {
  const date = new Date().toISOString().slice(0, 10);
  setDaily(date);
  el.seed.value = `daily ${date}`;
  regenerate({ animate: true });
}

el.seed.addEventListener("input", () => { setDaily(null); scheduleRegenerate(); });
el.seed.addEventListener("keydown", (e) => {
  if (e.key === "Enter") { regenerate({ animate: true }); el.seed.blur(); }
});
$("random").addEventListener("click", randomize);
el.daily.addEventListener("click", goDaily);

function buildSliders() {
  el.sliders.replaceChildren();
  let section = "map";
  let heading = null;
  el.recipe.value.split("\n").forEach((line, index) => {
    const use = line.match(/^\s*use\s*=\s*"([^"]+)"/);
    if (use) { section = use[1]; heading = null; return; }
    const m = line.match(NUMBER_LINE);
    if (!m || m[2] === "gen_version") return;
    if (!heading) {
      heading = document.createElement("h3");
      heading.textContent = section.replaceAll("_", " ");
      el.sliders.append(heading);
    }
    el.sliders.append(slider(m[2], m[4], index));
  });
}

function slider(key, raw, lineIndex) {
  const value = Number(raw);
  const isFloat = raw.includes(".");
  const isSize = key === "width" || key === "height";
  const [min, max, step] = isFloat
    ? (value <= 1 ? [0, 1, 0.01] : [0, Math.max(value * 2, 1), 0.5])
    : isSize ? [16, 200, 1] : [0, Math.max(value * 2, 10), 1];

  const wrap = document.createElement("label");
  wrap.className = "slider";
  const row = document.createElement("span");
  row.className = "slider-row";
  const name = document.createElement("span");
  name.textContent = key.replaceAll("_", " ");
  const out = document.createElement("output");
  out.textContent = raw;
  row.append(name, out);

  const input = document.createElement("input");
  Object.assign(input, { type: "range", min, max, step, value });
  const paintTrack = () => input.style.setProperty("--p", `${((input.value - min) / (max - min)) * 100}%`);
  paintTrack();
  input.addEventListener("input", () => {
    const text = isFloat ? String(Number(Number(input.value).toFixed(2))).replace(/^(\d+)$/, "$1.0") : input.value;
    out.textContent = text;
    paintTrack();
    const lines = el.recipe.value.split("\n");
    lines[lineIndex] = lines[lineIndex].replace(NUMBER_LINE, (_, a, k, eq, _v, rest) => `${a}${k}${eq}${text}${rest}`);
    el.recipe.value = lines.join("\n");
    scheduleRegenerate();
  });
  wrap.append(row, input);
  return wrap;
}

function togglePanel(force) {
  const collapse = force ?? !el.panel.hasAttribute("data-collapsed");
  el.panel.toggleAttribute("data-collapsed", collapse);
  el.panelToggle.setAttribute("aria-expanded", !collapse);
  if (cam.fitted) fit();
}

el.panelToggle.addEventListener("click", () => togglePanel());

let pending = 0;
function scheduleRegenerate() {
  cancelAnimationFrame(pending);
  pending = requestAnimationFrame(() => regenerate());
}

function openRecipe() {
  el.dialog.showModal();
  el.recipe.focus();
}

let typing = 0;
el.recipe.addEventListener("input", () => {
  clearTimeout(typing);
  typing = setTimeout(() => { buildSliders(); regenerate(); }, 200);
});
$("edit-recipe").addEventListener("click", openRecipe);
$("reset").addEventListener("click", () => {
  el.recipe.value = styleSource(state.style);
  buildSliders();
  regenerate();
});

function setView(view) {
  state.view = view;
  for (const b of document.querySelectorAll("[data-view]")) b.setAttribute("aria-pressed", b.dataset.view === view);
  el.canvas.hidden = view !== "tiles";
  el.ascii.hidden = view !== "ascii";
  if (view === "ascii") renderAscii();
  fit(false);
}

for (const b of document.querySelectorAll("[data-view]")) {
  b.addEventListener("click", () => setView(b.dataset.view));
}

$("share").addEventListener("click", async () => {
  writeHash();
  try {
    await navigator.clipboard.writeText(location.href);
    toast("Link copied");
  } catch {
    toast("Copy the link from the address bar");
  }
});

$("png").addEventListener("click", () => {
  buffer.toBlob((blob) => download(blob, `${fileStem()}.png`));
});

$("json").addEventListener("click", () => {
  download(new Blob([state.current.json()], { type: "application/json" }), `${fileStem()}.json`);
});

/** #style=dungeon&seed=hello, plus the recipe itself once it's been edited. */
function writeHash() {
  const params = new URLSearchParams({ style: state.style });
  if (state.daily) params.set("daily", state.daily);
  else params.set("seed", el.seed.value);
  if (el.recipe.value !== styleSource(state.style)) params.set("recipe", toBase64Url(el.recipe.value));
  history.replaceState(null, "", `#${params}`);
}

function readHash() {
  const params = new URLSearchParams(location.hash.slice(1));
  const style = state.styles.some((s) => s.name === params.get("style")) ? params.get("style") : state.styles[0].name;
  el.recipe.value = params.has("recipe") ? fromBase64Url(params.get("recipe")) : styleSource(style);
  setDaily(params.get("daily"));
  el.seed.value = state.daily ? `daily ${state.daily}` : params.get("seed") ?? randomSeed();
  return { style, keepRecipe: params.has("recipe") };
}

document.addEventListener("keydown", (e) => {
  if (e.metaKey || e.ctrlKey || e.altKey || e.target.closest?.("input, textarea, select, dialog")) return;
  const key = e.key.toLowerCase();
  const actions = {
    r: randomize, d: goDaily, f: () => fit(), t: () => togglePanel(), e: openRecipe,
    a: () => setView(state.view === "tiles" ? "ascii" : "tiles"),
    "+": () => zoomCenter(1.5), "=": () => zoomCenter(1.5), "-": () => zoomCenter(1 / 1.5),
  };
  const styleIndex = Number(key) - 1;
  if (actions[key]) actions[key]();
  else if (styleIndex >= 0 && styleIndex < state.styles.length) selectStyle(state.styles[styleIndex].name);
  else return;
  e.preventDefault();
});

function paintIcons() {
  if (state.sheet) paintTiles(state.sheet);
}

function shuffle(array) {
  for (let i = array.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [array[i], array[j]] = [array[j], array[i]];
  }
  return array;
}

function styleSource(name) {
  return state.styles.find((s) => s.name === name).source;
}

function randomSeed() {
  const pick = () => SEED_WORDS[Math.floor(Math.random() * SEED_WORDS.length)];
  return `${pick()}-${pick()}-${Math.floor(Math.random() * 1000)}`;
}

function toBase64Url(text) {
  const bytes = new TextEncoder().encode(text);
  return btoa(String.fromCharCode(...bytes)).replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

function fromBase64Url(b64) {
  const bin = atob(b64.replaceAll("-", "+").replaceAll("_", "/"));
  return new TextDecoder().decode(Uint8Array.from(bin, (c) => c.charCodeAt(0)));
}

function fileStem() {
  const seed = (state.daily ? `daily-${state.daily}` : el.seed.value).replace(/[^\w-]+/g, "_");
  return `undercroft-${state.style}-${seed}`;
}

function download(blob, name) {
  const a = Object.assign(document.createElement("a"), { href: URL.createObjectURL(blob), download: name });
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

// Start last so every declaration above is initialised.

buildStyleTabs();
if (narrow.matches) togglePanel(true);
const initial = readHash();
selectStyle(initial.style, { keepRecipe: initial.keepRecipe });
