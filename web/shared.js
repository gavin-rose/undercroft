export const TILE = 16;

export function loadImage(src) {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = reject;
    img.src = src;
  });
}

const sprites = new Map();

/** One tile from the sheet, tinted `[r, g, b]` or any CSS colour. Cached. */
export function sprite(sheet, [col, row], color) {
  const fill = Array.isArray(color) ? `rgb(${color})` : color;
  const key = `${col},${row},${fill}`;
  let canvas = sprites.get(key);
  if (!canvas) {
    canvas = Object.assign(document.createElement("canvas"), { width: TILE, height: TILE });
    const c = canvas.getContext("2d");
    c.drawImage(sheet, col * TILE, row * TILE, TILE, TILE, 0, 0, TILE, TILE);
    c.globalCompositeOperation = "source-in";
    c.fillStyle = fill;
    c.fillRect(0, 0, TILE, TILE);
    sprites.set(key, canvas);
  }
  return canvas;
}

/** Paints every `canvas.tile[data-tile]` in its parent's text colour, or the accent. */
export function paintTiles(sheet) {
  const accent = getComputedStyle(document.documentElement).getPropertyValue("--accent").trim();
  for (const canvas of document.querySelectorAll("canvas.tile[data-tile]")) {
    const tile = canvas.dataset.tile.split(",").map(Number);
    const color = canvas.dataset.color === "accent" ? accent : getComputedStyle(canvas.parentElement).color;
    const c = canvas.getContext("2d");
    c.clearRect(0, 0, TILE, TILE);
    c.drawImage(sprite(sheet, tile, color), 0, 0);
  }
}

let toastTimer = 0;
export function toast(text) {
  const el = document.getElementById("toast");
  el.textContent = text;
  el.hidden = false;
  el.style.animation = "none";
  void el.offsetWidth; // restart the pop animation
  el.style.animation = "";
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { el.hidden = true; }, 1400);
}
