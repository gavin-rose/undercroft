import init, { Play } from "./pkg/undercroft_wasm.js";
import { TILE, loadImage, paintTiles, sprite, toast } from "./shared.js";

const SITE = "undercroft.debugr.net/play.html";
const PLAYER_SPRITE = [27, 0];
const KEYS = {
  ArrowUp: "up", w: "up", k: "up",
  ArrowDown: "down", s: "down", j: "down",
  ArrowLeft: "left", a: "left", h: "left",
  ArrowRight: "right", d: "right", l: "right",
  " ": "wait", ".": "wait", Enter: "descend", ">": "descend", q: "quaff",
};

const $ = (id) => document.getElementById(id);
const el = {
  screen: $("screen"), hurt: $("hurt"), mode: $("mode"), hpText: $("hp-text"), hpFill: $("hp-fill"),
  floor: $("floor"), atk: $("atk"), def: $("def"), gold: $("gold"), potions: $("potions"), turn: $("turn"),
  pips: $("pips"), log: $("log"), prompt: $("prompt"), end: $("end"), endIcon: $("end-icon"),
  endTitle: $("end-title"), endSub: $("end-sub"), shareText: $("share-text"), endNote: $("end-note"),
};
const ctx = el.screen.getContext("2d");
const floorCanvas = document.createElement("canvas");
const fogCanvas = document.createElement("canvas");
const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");
const style = getComputedStyle(document.documentElement);
const INK = style.getPropertyValue("--ink").trim();
const ACCENT = style.getPropertyValue("--accent").trim();

await init();
const sheet = await loadImage("pkg/tiles.png");
paintTiles(sheet);

let mode;
let game = null;
let view = null;
let floorDepth = 0;
let locked = false;
let scale = 3;
const cam = { x: 0, y: 0 };

// Game flow

function start(next) {
  mode = next;
  game?.free();
  game = mode.daily ? Play.daily(mode.date) : Play.seeded(mode.seed);
  floorDepth = 0;
  locked = false;
  el.mode.replaceChildren(mode.daily ? "Daily " : "Practice ", Object.assign(document.createElement("b"), {
    textContent: mode.daily ? mode.date : mode.seed,
  }));
  history.replaceState(null, "", mode.daily ? location.pathname : `#seed=${encodeURIComponent(mode.seed)}`);
  refresh();

  const done = mode.daily && loadResult(mode.date);
  if (done) {
    locked = true;
    showEnd(done);
  }
}

function act(name) {
  if (locked) return;
  const hpBefore = view.hp;
  game.act(name);
  refresh();
  if (view.hp < hpBefore) hurt();
  if (view.status !== "playing") finish();
}

function refresh() {
  view = JSON.parse(game.view());
  if (view.depth !== floorDepth) {
    loadFloor();
    snapCamera();
  }
  buildFog();
  renderHud();
  kick();
}

function finish() {
  locked = true;
  const result = {
    won: view.status === "won",
    depth: view.depth,
    floors: view.floors,
    turns: view.turn,
    kills: view.kills,
    gold: view.gold,
    score: view.score,
    killedBy: view.killed_by,
    replay: game.replay(),
  };
  if (mode.daily) saveResult(mode.date, result);
  setTimeout(() => showEnd(result), 600);
}

function showEnd(result) {
  el.endTitle.textContent = result.won ? "You escaped the undercroft" : `Slain by a ${result.killedBy}`;
  el.endSub.textContent = result.won
    ? `All ${result.floors} floors in ${result.turns} turns. Score ${result.score}.`
    : `On floor ${result.depth} of ${result.floors} after ${result.turns} turns. Score ${result.score}.`;
  el.endIcon.getContext("2d").clearRect(0, 0, TILE, TILE);
  el.endIcon.getContext("2d").drawImage(sprite(sheet, result.won ? [40, 16] : [34, 12], result.won ? ACCENT : "#ff6b57"), 0, 0);
  el.shareText.textContent = shareText(result);
  el.endNote.textContent = mode.daily
    ? "A new dungeon opens every day at midnight UTC."
    : "Practice runs don't count toward the daily.";
  if (!el.end.open) el.end.showModal();
}

function shareText(r) {
  const head = mode.daily ? `undercroft daily ${mode.date}` : `undercroft practice "${mode.seed}"`;
  const outcome = r.won ? `🏆 escaped all ${r.floors} floors` : `💀 floor ${r.depth}/${r.floors}, slain by a ${r.killedBy}`;
  const reached = r.won ? r.floors : r.depth;
  const bar = "▰".repeat(reached) + "▱".repeat(r.floors - reached);
  const link = mode.daily ? SITE : `${SITE}#seed=${encodeURIComponent(mode.seed)}`;
  return `${head}\n${outcome}\n${bar} ${r.turns} turns · ${r.kills} kills · ${r.gold} gold\n${link}`;
}

function practice() {
  el.end.close();
  start({ daily: false, seed: randomSeed() });
}

// Drawing

function loadFloor() {
  floorDepth = view.depth;
  const pixels = game.floorPixels();
  const w = view.width * TILE;
  const h = view.height * TILE;
  Object.assign(floorCanvas, { width: w, height: h });
  floorCanvas.getContext("2d").putImageData(new ImageData(new Uint8ClampedArray(pixels.buffer, pixels.byteOffset, pixels.length), w, h), 0, 0);
  Object.assign(fogCanvas, { width: view.width, height: view.height });
}

/** One pixel per tile: opaque where unseen, dimmed where remembered. */
function buildFog() {
  const fog = new ImageData(view.width, view.height);
  const [r, g, b] = [12, 10, 16];
  for (let i = 0; i < view.fog.length; i++) {
    fog.data.set([r, g, b, view.fog[i] === "0" ? 255 : view.fog[i] === "1" ? 170 : 0], i * 4);
  }
  fogCanvas.getContext("2d").putImageData(fog, 0, 0);
}

function resize() {
  const dpr = devicePixelRatio || 1;
  el.screen.width = Math.round(innerWidth * dpr);
  el.screen.height = Math.round(innerHeight * dpr);
  scale = Math.min(5, Math.max(2, Math.floor(Math.min(innerWidth / (TILE * 20), innerHeight / (TILE * 13)))));
  kick();
}

function draw() {
  const dpr = devicePixelRatio || 1;
  const s = scale * dpr;
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.fillStyle = INK;
  ctx.fillRect(0, 0, el.screen.width, el.screen.height);
  ctx.setTransform(s, 0, 0, s, Math.round(el.screen.width / 2 - cam.x * s), Math.round(el.screen.height / 2 - cam.y * s));
  ctx.imageSmoothingEnabled = false;

  ctx.drawImage(floorCanvas, 0, 0);
  ctx.drawImage(fogCanvas, 0, 0, view.width * TILE, view.height * TILE);

  for (const item of view.items) drawSprite(item.sprite, item.color, item.pos);
  for (const m of view.monsters) {
    drawSprite(m.sprite, m.color, m.pos);
    const [hp, max] = m.hp;
    if (hp < max) {
      ctx.fillStyle = "#000";
      ctx.fillRect(m.pos.x * TILE + 1, m.pos.y * TILE - 3, TILE - 2, 3);
      ctx.fillStyle = "#ff5a4d";
      ctx.fillRect(m.pos.x * TILE + 2, m.pos.y * TILE - 2, Math.max(1, Math.round((TILE - 4) * hp / max)), 1);
    }
  }
  if (view.status !== "dead") drawSprite(PLAYER_SPRITE, ACCENT, view.player);
}

function drawSprite(tile, color, pos) {
  const x = pos.x * TILE;
  const y = pos.y * TILE;
  ctx.drawImage(sprite(sheet, tile, "#000"), x + 1, y + 1);
  ctx.drawImage(sprite(sheet, tile, color), x, y);
}

function target() {
  return { x: view.player.x * TILE + TILE / 2, y: view.player.y * TILE + TILE / 2 };
}

function snapCamera() {
  Object.assign(cam, target());
}

let frame = 0;
function kick() {
  if (!frame) frame = requestAnimationFrame(tick);
}

function tick() {
  frame = 0;
  const t = target();
  const ease = reducedMotion.matches ? 1 : 0.3;
  cam.x += (t.x - cam.x) * ease;
  cam.y += (t.y - cam.y) * ease;
  if (Math.abs(t.x - cam.x) < 0.5 && Math.abs(t.y - cam.y) < 0.5) Object.assign(cam, t);
  else kick();
  draw();
}

function hurt() {
  el.hurt.classList.add("on");
  requestAnimationFrame(() => requestAnimationFrame(() => el.hurt.classList.remove("on")));
}

// HUD

function renderHud() {
  el.hpText.textContent = `${view.hp}/${view.max_hp}`;
  el.hpFill.style.width = `${(view.hp / view.max_hp) * 100}%`;
  el.hpFill.parentElement.classList.toggle("low", view.hp / view.max_hp < 0.3);
  el.floor.textContent = `${view.depth}/${view.floors}`;
  el.atk.textContent = view.atk;
  el.def.textContent = view.def;
  el.gold.textContent = view.gold;
  el.potions.textContent = view.potions;
  el.turn.textContent = view.turn;
  el.pips.replaceChildren(...Array.from({ length: view.floors }, (_, i) => {
    const pip = document.createElement("span");
    if (i + 1 < view.depth || view.status === "won") pip.className = "done";
    else if (i + 1 === view.depth) pip.className = "here";
    return pip;
  }));
  el.log.replaceChildren(...view.log.map((line) => Object.assign(document.createElement("li"), { textContent: line })));
  el.prompt.hidden = !(view.on_stairs && view.status === "playing");
}

// Input

document.addEventListener("keydown", (e) => {
  if (el.end.open || e.metaKey || e.ctrlKey || e.altKey) return;
  if (locked) {
    el.end.showModal();
    return;
  }
  const action = KEYS[e.key.length === 1 ? e.key.toLowerCase() : e.key];
  if (!action) return;
  e.preventDefault();
  act(action);
});

for (const button of document.querySelectorAll("[data-act]")) {
  button.addEventListener("pointerdown", (e) => {
    e.preventDefault();
    act(button.dataset.act);
  });
}

$("copy").addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText(el.shareText.textContent);
    toast("Result copied");
  } catch {
    toast("Select the text to copy it");
  }
});
$("practice").addEventListener("click", practice);
addEventListener("resize", resize);

// Storage is a convenience: the game still works without it.

function loadResult(date) {
  try {
    return JSON.parse(localStorage.getItem(`undercroft:daily:${date}`));
  } catch {
    return null;
  }
}

function saveResult(date, result) {
  try {
    localStorage.setItem(`undercroft:daily:${date}`, JSON.stringify(result));
  } catch {}
}

function randomSeed() {
  const words = ["owl", "moss", "ember", "crypt", "lantern", "hollow", "rune", "ash", "vault", "fern", "tide", "bone"];
  const pick = () => words[Math.floor(Math.random() * words.length)];
  return `${pick()}-${pick()}-${Math.floor(Math.random() * 1000)}`;
}

// Start last so every declaration above is initialised.
resize();
const seed = new URLSearchParams(location.hash.slice(1)).get("seed");
start(seed ? { daily: false, seed } : { daily: true, date: new Date().toISOString().slice(0, 10) });
