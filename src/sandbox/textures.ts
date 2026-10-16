import * as THREE from "three";

function canvas(w: number, h: number): [HTMLCanvasElement, CanvasRenderingContext2D] {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  return [c, c.getContext("2d") as CanvasRenderingContext2D];
}

function tex(c: HTMLCanvasElement, repeat = false): THREE.CanvasTexture {
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  t.anisotropy = 4;
  if (repeat) {
    t.wrapS = THREE.RepeatWrapping;
    t.wrapT = THREE.RepeatWrapping;
  }
  return t;
}

let seed = 1;
function rnd(): number {
  seed = (seed * 16807) % 2147483647;
  return (seed - 1) / 2147483646;
}

/** Facade texture: rows of windows, some lit (emissive map uses the same layout). */
export function facadeTextures(): { map: THREE.Texture; emissive: THREE.Texture } {
  const W = 256;
  const H = 512;
  const [c, g] = canvas(W, H);
  const [e, ge] = canvas(W, H);
  g.fillStyle = "#5a5d68";
  g.fillRect(0, 0, W, H);
  ge.fillStyle = "#000";
  ge.fillRect(0, 0, W, H);
  const cols = 8;
  const rows = 8;
  const cw = W / cols;
  const rh = H / rows;
  const warm = ["#ffcf8a", "#ffe0b0", "#ffd27a", "#bfe6ff", "#ff9ad5", "#9affef"];
  for (let r = 0; r < rows; r++) {
    for (let k = 0; k < cols; k++) {
      const ww = cw * 0.52;
      const wh = rh * 0.42;
      const x = k * cw + (cw - ww) / 2;
      const y = r * rh + rh * 0.22;
      g.fillStyle = "#1a2230";
      g.fillRect(x, y, ww, wh);
      // Floor slab line.
      g.fillStyle = "#454852";
      g.fillRect(k * cw, r * rh, cw, 3);
      if (rnd() < 0.3) {
        const col = warm[Math.floor(rnd() * warm.length)];
        ge.fillStyle = col;
        ge.globalAlpha = 0.18 + rnd() * 0.42;
        ge.fillRect(x, y, ww, wh);
        ge.globalAlpha = 1;
        g.fillStyle = "#3a3f4a";
        g.fillRect(x, y, ww, wh);
      }
    }
  }
  const map = tex(c, true);
  const emissive = tex(e, true);
  return { map, emissive };
}

/** Dark asphalt with grain. */
export function asphaltTexture(): THREE.Texture {
  const [c, g] = canvas(256, 256);
  g.fillStyle = "#24252b";
  g.fillRect(0, 0, 256, 256);
  const img = g.getImageData(0, 0, 256, 256);
  for (let i = 0; i < img.data.length; i += 4) {
    const n = (rnd() - 0.5) * 26;
    img.data[i] += n;
    img.data[i + 1] += n;
    img.data[i + 2] += n + 2;
  }
  g.putImageData(img, 0, 0);
  // Patches and cracks.
  g.globalAlpha = 0.25;
  for (let i = 0; i < 18; i++) {
    g.fillStyle = rnd() < 0.5 ? "#1b1c21" : "#2e2f36";
    g.beginPath();
    g.ellipse(rnd() * 256, rnd() * 256, 10 + rnd() * 30, 6 + rnd() * 18, rnd() * 3, 0, Math.PI * 2);
    g.fill();
  }
  const t = tex(c, true);
  t.repeat.set(60, 60);
  return t;
}

export function concreteTexture(): THREE.Texture {
  const [c, g] = canvas(128, 128);
  g.fillStyle = "#4a4b52";
  g.fillRect(0, 0, 128, 128);
  g.strokeStyle = "#3a3b41";
  g.lineWidth = 2;
  for (let i = 0; i <= 128; i += 32) {
    g.beginPath();
    g.moveTo(i, 0);
    g.lineTo(i, 128);
    g.moveTo(0, i);
    g.lineTo(128, i);
    g.stroke();
  }
  const img = g.getImageData(0, 0, 128, 128);
  for (let i = 0; i < img.data.length; i += 4) {
    const n = (rnd() - 0.5) * 14;
    img.data[i] += n;
    img.data[i + 1] += n;
    img.data[i + 2] += n;
  }
  g.putImageData(img, 0, 0);
  return tex(c, true);
}

/** Soft radial glow (for lamps, headlight pools). */
export function glowTexture(): THREE.Texture {
  const [c, g] = canvas(128, 128);
  const grad = g.createRadialGradient(64, 64, 0, 64, 64, 64);
  grad.addColorStop(0, "rgba(255,255,255,1)");
  grad.addColorStop(0.25, "rgba(255,255,255,0.55)");
  grad.addColorStop(0.6, "rgba(255,255,255,0.12)");
  grad.addColorStop(1, "rgba(255,255,255,0)");
  g.fillStyle = grad;
  g.fillRect(0, 0, 128, 128);
  return tex(c);
}

/** Headlight beam pool on the road: elongated, bright near the car. */
export function beamTexture(): THREE.Texture {
  const [c, g] = canvas(128, 256);
  g.save();
  g.translate(64, 128);
  g.scale(1, 2);
  const grad = g.createRadialGradient(0, 30, 2, 0, 0, 62);
  grad.addColorStop(0, "rgba(255,255,255,0.95)");
  grad.addColorStop(0.45, "rgba(255,255,255,0.4)");
  grad.addColorStop(1, "rgba(255,255,255,0)");
  g.fillStyle = grad;
  g.fillRect(-64, -64, 128, 128);
  g.restore();
  return tex(c);
}

/** White alpha masks for signal lamps. */
export function lampMask(kind: "ball" | "arrow" | "walk" | "hand"): THREE.Texture {
  const [c, g] = canvas(64, 64);
  g.fillStyle = "#fff";
  if (kind === "ball") {
    g.beginPath();
    g.arc(32, 32, 29, 0, Math.PI * 2);
    g.fill();
  } else if (kind === "arrow") {
    g.beginPath();
    g.moveTo(6, 32);
    g.lineTo(30, 10);
    g.lineTo(30, 23);
    g.lineTo(58, 23);
    g.lineTo(58, 41);
    g.lineTo(30, 41);
    g.lineTo(30, 54);
    g.closePath();
    g.fill();
  } else if (kind === "walk") {
    g.beginPath();
    g.arc(34, 12, 7, 0, Math.PI * 2);
    g.fill();
    g.lineWidth = 7;
    g.strokeStyle = "#fff";
    g.lineCap = "round";
    g.beginPath();
    g.moveTo(33, 22);
    g.lineTo(30, 40);
    g.lineTo(20, 58);
    g.moveTo(30, 40);
    g.lineTo(42, 58);
    g.moveTo(32, 26);
    g.lineTo(20, 36);
    g.moveTo(32, 26);
    g.lineTo(44, 34);
    g.stroke();
  } else {
    // Raised hand.
    g.beginPath();
    g.roundRect(18, 26, 30, 30, 8);
    g.fill();
    for (let i = 0; i < 4; i++) g.fillRect(19 + i * 7.5, 8 + (i === 0 || i === 3 ? 6 : 0), 6, 24);
    g.fillRect(44, 28, 12, 6);
  }
  return tex(c);
}

/** Road-sign faces drawn in the style of US/MUTCD signage. */
export function signTexture(kind: string, text?: string): THREE.Texture {
  const font = "'Overpass', 'Highway Gothic', 'Arial Narrow', sans-serif";
  if (kind === "speed") {
    const [c, g] = canvas(128, 160);
    g.fillStyle = "#f4f4f0";
    g.fillRect(0, 0, 128, 160);
    g.strokeStyle = "#111";
    g.lineWidth = 6;
    g.strokeRect(6, 6, 116, 148);
    g.fillStyle = "#111";
    g.textAlign = "center";
    g.font = `700 22px ${font}`;
    g.fillText("SPEED", 64, 38);
    g.fillText("LIMIT", 64, 62);
    g.font = `800 64px ${font}`;
    g.fillText(text ?? "40", 64, 132);
    return tex(c);
  }
  if (kind === "stop") {
    const [c, g] = canvas(128, 128);
    g.fillStyle = "#c8102e";
    g.beginPath();
    for (let i = 0; i < 8; i++) {
      const a = Math.PI / 8 + (i * Math.PI) / 4;
      g.lineTo(64 + Math.cos(a) * 62, 64 + Math.sin(a) * 62);
    }
    g.closePath();
    g.fill();
    g.strokeStyle = "#fff";
    g.lineWidth = 4;
    g.beginPath();
    for (let i = 0; i < 8; i++) {
      const a = Math.PI / 8 + (i * Math.PI) / 4;
      g.lineTo(64 + Math.cos(a) * 56, 64 + Math.sin(a) * 56);
    }
    g.closePath();
    g.stroke();
    g.fillStyle = "#fff";
    g.textAlign = "center";
    g.font = `800 38px ${font}`;
    g.fillText("STOP", 64, 78);
    return tex(c);
  }
  if (kind === "yield") {
    const [c, g] = canvas(128, 128);
    g.fillStyle = "#c8102e";
    g.beginPath();
    g.moveTo(4, 8);
    g.lineTo(124, 8);
    g.lineTo(64, 120);
    g.closePath();
    g.fill();
    g.fillStyle = "#fff";
    g.beginPath();
    g.moveTo(28, 22);
    g.lineTo(100, 22);
    g.lineTo(64, 90);
    g.closePath();
    g.fill();
    g.fillStyle = "#c8102e";
    g.textAlign = "center";
    g.font = `800 17px ${font}`;
    g.fillText("YIELD", 64, 44);
    return tex(c);
  }
  if (kind === "oneway") {
    const [c, g] = canvas(192, 64);
    g.fillStyle = "#111";
    g.fillRect(0, 0, 192, 64);
    g.fillStyle = "#fff";
    g.beginPath();
    g.moveTo(10, 32);
    g.lineTo(40, 8);
    g.lineTo(40, 20);
    g.lineTo(182, 20);
    g.lineTo(182, 44);
    g.lineTo(40, 44);
    g.lineTo(40, 56);
    g.closePath();
    g.fill();
    g.fillStyle = "#111";
    g.font = `800 20px ${font}`;
    g.textAlign = "center";
    g.fillText("ONE WAY", 112, 40);
    return tex(c);
  }
  if (kind === "street") {
    const [c, g] = canvas(256, 48);
    g.fillStyle = "#0b6b3a";
    g.fillRect(0, 0, 256, 48);
    g.strokeStyle = "#fff";
    g.lineWidth = 2;
    g.strokeRect(3, 3, 250, 42);
    g.fillStyle = "#fff";
    g.font = `700 26px ${font}`;
    g.textAlign = "center";
    g.fillText((text ?? "").toUpperCase(), 128, 34, 240);
    return tex(c);
  }
  if (kind === "camera") {
    const [c, g] = canvas(96, 128);
    g.fillStyle = "#f4f4f0";
    g.fillRect(0, 0, 96, 128);
    g.strokeStyle = "#111";
    g.lineWidth = 4;
    g.strokeRect(4, 4, 88, 120);
    g.fillStyle = "#111";
    g.textAlign = "center";
    g.font = `700 13px ${font}`;
    g.fillText("PHOTO", 48, 28);
    g.fillText("ENFORCED", 48, 46);
    g.fillRect(28, 62, 40, 26);
    g.beginPath();
    g.arc(48, 75, 9, 0, Math.PI * 2);
    g.fillStyle = "#f4f4f0";
    g.fill();
    g.fillStyle = "#111";
    g.font = `700 12px ${font}`;
    g.fillText(text ?? "RED LIGHT", 48, 112);
    return tex(c);
  }
  throw new Error(`unknown sign ${kind}`);
}

/** Painted lane-use arrow (white on transparent). */
export function laneArrowTexture(turns: string[]): THREE.Texture {
  const [c, g] = canvas(64, 128);
  g.strokeStyle = "#fff";
  g.fillStyle = "#fff";
  g.lineWidth = 9;
  g.lineCap = "butt";
  const stemX = 32;
  g.beginPath();
  g.moveTo(stemX, 124);
  g.lineTo(stemX, 60);
  g.stroke();
  const head = (x: number, y: number, ang: number) => {
    g.save();
    g.translate(x, y);
    g.rotate(ang);
    g.beginPath();
    g.moveTo(0, -18);
    g.lineTo(14, 4);
    g.lineTo(-14, 4);
    g.closePath();
    g.fill();
    g.restore();
  };
  if (turns.includes("straight")) {
    g.beginPath();
    g.moveTo(stemX, 60);
    g.lineTo(stemX, 22);
    g.stroke();
    head(stemX, 18, 0);
  }
  if (turns.includes("left")) {
    g.beginPath();
    g.moveTo(stemX, 72);
    g.quadraticCurveTo(stemX, 48, 14, 48);
    g.stroke();
    head(10, 48, -Math.PI / 2);
  }
  if (turns.includes("right")) {
    g.beginPath();
    g.moveTo(stemX, 72);
    g.quadraticCurveTo(stemX, 48, 50, 48);
    g.stroke();
    head(54, 48, Math.PI / 2);
  }
  return tex(c);
}

/** Neon shop sign. */
export function neonTexture(word: string, color: string): THREE.Texture {
  const [c, g] = canvas(256, 64);
  g.fillStyle = "#050507";
  g.fillRect(0, 0, 256, 64);
  g.font = "700 40px 'Big Shoulders Display', 'Arial Narrow', sans-serif";
  g.textAlign = "center";
  g.shadowColor = color;
  g.shadowBlur = 14;
  g.fillStyle = color;
  g.fillText(word, 128, 46, 240);
  g.shadowBlur = 0;
  g.fillStyle = "#fff";
  g.globalAlpha = 0.6;
  g.fillText(word, 128, 46, 240);
  return tex(c);
}

export function smokeTexture(): THREE.Texture {
  const [c, g] = canvas(64, 64);
  const grad = g.createRadialGradient(32, 32, 0, 32, 32, 32);
  grad.addColorStop(0, "rgba(255,255,255,0.6)");
  grad.addColorStop(1, "rgba(255,255,255,0)");
  g.fillStyle = grad;
  g.fillRect(0, 0, 64, 64);
  return tex(c);
}
