import { motion } from "motion/react";
import type { SimpleIcon } from "simple-icons";
import {
  siAcer,
  siAlienware,
  siAmd,
  siApple,
  siArm,
  siAsus,
  siBosch,
  siBroadcom,
  siCisco,
  siCoolermaster,
  siCorsair,
  siDeepcool,
  siDell,
  siDji,
  siElgato,
  siEpson,
  siEricsson,
  siEspressif,
  siFluke,
  siFramework,
  siFujitsu,
  siHitachi,
  siHonor,
  siHp,
  siHuawei,
  siHyperx,
  siIntel,
  siKingstontechnology,
  siLenovo,
  siLg,
  siLinksys,
  siMediatek,
  siMotorola,
  siMsi,
  siNec,
  siNetgear,
  siNokia,
  siNordicsemiconductor,
  siNvidia,
  siNxp,
  siNzxt,
  siOneplus,
  siOppo,
  siPanasonic,
  siProxmox,
  siQemu,
  siQnap,
  siQualcomm,
  siRaspberrypi,
  siRazer,
  siRepublicofgamers,
  siSamsung,
  siSeagate,
  siSharp,
  siSiemens,
  siSnapdragon,
  siSony,
  siSteelseries,
  siStmicroelectronics,
  siSupermicro,
  siSynology,
  siToshiba,
  siTplink,
  siVirtualbox,
  siVivo,
  siVmware,
  siXiaomi,
} from "simple-icons";

export type VendorId =
  | "nvidia"
  | "amd"
  | "intel"
  | "rog"
  | "asus"
  | "virtualbox"
  | "lg"
  | "samsung"
  | "corsair"
  | "hyperx"
  | "kingston"
  | "msi"
  | "seagate"
  | "toshiba"
  | "snapdragon"
  | "qualcomm"
  | "mediatek"
  | "broadcom"
  | "apple"
  | "alienware"
  | "dell"
  | "hp"
  | "lenovo"
  | "acer"
  | "razer"
  | "steelseries"
  | "coolermaster"
  | "nzxt"
  | "deepcool"
  | "netgear"
  | "linksys"
  | "tplink"
  | "synology"
  | "qnap"
  | "supermicro"
  | "framework"
  | "vmware"
  | "proxmox"
  | "qemu"
  | "raspberrypi"
  | "arm"
  | "fujitsu"
  | "hitachi"
  | "nec"
  | "panasonic"
  | "sharp"
  | "xiaomi"
  | "huawei"
  | "honor"
  | "oneplus"
  | "oppo"
  | "vivo"
  | "motorola"
  | "nokia"
  | "cisco"
  | "sony"
  | "epson"
  | "elgato"
  | "dji"
  | "bosch"
  | "siemens"
  | "ericsson"
  | "espressif"
  | "nordic"
  | "stmicro"
  | "nxp"
  | "fluke";

/** @deprecated Use VendorId. Kept so older imports still type-check. */
export type GpuVendor = VendorId;

const ICONS: Record<VendorId, SimpleIcon> = {
  nvidia: siNvidia,
  amd: siAmd,
  intel: siIntel,
  rog: siRepublicofgamers,
  asus: siAsus,
  virtualbox: siVirtualbox,
  lg: siLg,
  samsung: siSamsung,
  corsair: siCorsair,
  hyperx: siHyperx,
  kingston: siKingstontechnology,
  msi: siMsi,
  seagate: siSeagate,
  toshiba: siToshiba,
  snapdragon: siSnapdragon,
  qualcomm: siQualcomm,
  mediatek: siMediatek,
  broadcom: siBroadcom,
  apple: siApple,
  alienware: siAlienware,
  dell: siDell,
  hp: siHp,
  lenovo: siLenovo,
  acer: siAcer,
  razer: siRazer,
  steelseries: siSteelseries,
  coolermaster: siCoolermaster,
  nzxt: siNzxt,
  deepcool: siDeepcool,
  netgear: siNetgear,
  linksys: siLinksys,
  tplink: siTplink,
  synology: siSynology,
  qnap: siQnap,
  supermicro: siSupermicro,
  framework: siFramework,
  vmware: siVmware,
  proxmox: siProxmox,
  qemu: siQemu,
  raspberrypi: siRaspberrypi,
  arm: siArm,
  fujitsu: siFujitsu,
  hitachi: siHitachi,
  nec: siNec,
  panasonic: siPanasonic,
  sharp: siSharp,
  xiaomi: siXiaomi,
  huawei: siHuawei,
  honor: siHonor,
  oneplus: siOneplus,
  oppo: siOppo,
  vivo: siVivo,
  motorola: siMotorola,
  nokia: siNokia,
  cisco: siCisco,
  sony: siSony,
  epson: siEpson,
  elgato: siElgato,
  dji: siDji,
  bosch: siBosch,
  siemens: siSiemens,
  ericsson: siEricsson,
  espressif: siEspressif,
  nordic: siNordicsemiconductor,
  stmicro: siStmicroelectronics,
  nxp: siNxp,
  fluke: siFluke,
};

type Rule = {
  id: VendorId;
  phrases?: string[];
  tokens?: string[];
  pnp?: string[];
};

const RULES: Rule[] = [
  { id: "nvidia", phrases: ["nvidia", "geforce", "quadro"], tokens: ["rtx", "gtx", "tesla"] },
  {
    id: "amd",
    phrases: ["advanced micro devices", "authenticamd", "radeon", "ryzen", "threadripper"],
    tokens: ["amd", "epyc", "athlon"],
  },
  {
    id: "intel",
    phrases: ["genuineintel", "intel", "uhd graphics", "iris xe", "iris plus", "iris graphics", "intel arc"],
    tokens: ["iris"],
  },
  { id: "rog", phrases: ["republic of gamers"], tokens: ["rog"] },
  { id: "asus", phrases: ["asustek", "asus"], pnp: ["aci", "aus"] },
  { id: "virtualbox", phrases: ["virtualbox", "virtual box", "oracle"], tokens: ["vbox"] },
  { id: "lg", phrases: ["lg display", "lg electronics"], tokens: ["lg"], pnp: ["gsm", "lgd"] },
  { id: "samsung", phrases: ["samsung"], pnp: ["sam", "sec", "sdc"] },
  { id: "corsair", phrases: ["corsair"] },
  { id: "hyperx", phrases: ["hyperx", "hyper x"] },
  { id: "kingston", phrases: ["kingston"] },
  { id: "msi", tokens: ["msi"], pnp: ["msi"] },
  { id: "seagate", phrases: ["seagate", "barracuda", "ironwolf", "firecuda"] },
  { id: "toshiba", phrases: ["toshiba"], pnp: ["tsb"] },
  { id: "snapdragon", phrases: ["snapdragon"] },
  { id: "qualcomm", phrases: ["qualcomm", "atheros"] },
  { id: "mediatek", phrases: ["mediatek", "media tek"] },
  { id: "broadcom", phrases: ["broadcom"] },
  { id: "apple", phrases: ["apple"], pnp: ["app"] },
  { id: "alienware", phrases: ["alienware"] },
  { id: "dell", phrases: ["dell"], pnp: ["del"] },
  { id: "hp", phrases: ["hewlett", "hewlett-packard"], tokens: ["hp"], pnp: ["hpn", "hwp", "hpq"] },
  { id: "lenovo", phrases: ["lenovo", "thinkpad"], pnp: ["len"] },
  { id: "acer", phrases: ["acer"], pnp: ["acr"] },
  { id: "razer", phrases: ["razer"] },
  { id: "steelseries", phrases: ["steelseries", "steel series"] },
  { id: "coolermaster", phrases: ["coolermaster", "cooler master"] },
  { id: "nzxt", tokens: ["nzxt"] },
  { id: "deepcool", phrases: ["deepcool", "deep cool"] },
  { id: "netgear", phrases: ["netgear"] },
  { id: "linksys", phrases: ["linksys"] },
  { id: "tplink", phrases: ["tp-link", "tplink", "tp link"] },
  { id: "synology", phrases: ["synology"] },
  { id: "qnap", tokens: ["qnap"] },
  { id: "supermicro", phrases: ["supermicro", "super micro"] },
  { id: "framework", phrases: ["framework computer"], tokens: ["framework"] },
  { id: "vmware", phrases: ["vmware"] },
  { id: "proxmox", phrases: ["proxmox"] },
  { id: "qemu", tokens: ["qemu"] },
  { id: "raspberrypi", phrases: ["raspberry pi", "raspberrypi"] },
  { id: "arm", tokens: ["arm", "arm64"] },
  { id: "fujitsu", phrases: ["fujitsu"], pnp: ["fuj"] },
  { id: "hitachi", phrases: ["hitachi"], pnp: ["hit"] },
  { id: "nec", tokens: ["nec"], pnp: ["nec"] },
  { id: "panasonic", phrases: ["panasonic", "matsushita"], pnp: ["mei"] },
  { id: "sharp", phrases: ["sharp"], pnp: ["shp"] },
  { id: "xiaomi", phrases: ["xiaomi"] },
  { id: "huawei", phrases: ["huawei"] },
  { id: "honor", tokens: ["honor"] },
  { id: "oneplus", phrases: ["oneplus", "one plus"] },
  { id: "oppo", tokens: ["oppo"] },
  { id: "vivo", tokens: ["vivo"] },
  { id: "motorola", phrases: ["motorola"] },
  { id: "nokia", phrases: ["nokia"] },
  { id: "cisco", phrases: ["cisco"] },
  { id: "sony", phrases: ["sony"], pnp: ["sny"] },
  { id: "epson", phrases: ["epson"] },
  { id: "elgato", phrases: ["elgato"] },
  { id: "dji", tokens: ["dji"] },
  { id: "bosch", phrases: ["bosch"] },
  { id: "siemens", phrases: ["siemens"] },
  { id: "ericsson", phrases: ["ericsson"] },
  { id: "espressif", phrases: ["espressif"] },
  { id: "nordic", phrases: ["nordic semiconductor"] },
  { id: "stmicro", phrases: ["stmicroelectronics", "st micro"] },
  { id: "nxp", tokens: ["nxp"] },
  { id: "fluke", phrases: ["fluke"] },
];

function escapeRe(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function hasPhrase(blob: string, phrase: string): boolean {
  return blob.includes(phrase);
}

function hasToken(blob: string, token: string): boolean {
  const body = escapeRe(token).replace(/\\ /g, "[\\s_-]+");
  return new RegExp(`(?:^|[^a-z0-9])${body}(?:[^a-z0-9]|$)`).test(blob);
}

function hasPnp(blob: string, id: string): boolean {
  return new RegExp(`(?:^|[^a-z0-9])${escapeRe(id)}(?:[0-9a-f]{4})?(?:[^a-z0-9]|$)`).test(blob);
}

function matchesRule(blob: string, rule: Rule): boolean {
  if (rule.phrases?.some((p) => hasPhrase(blob, p))) return true;
  if (rule.tokens?.some((t) => hasToken(blob, t))) return true;
  if (rule.pnp?.some((id) => hasPnp(blob, id))) return true;
  return false;
}

export function detectVendor(...parts: Array<string | null | undefined>): VendorId | null {
  const blob = parts
    .filter((p): p is string => Boolean(p && p.trim()))
    .join(" ")
    .toLowerCase()
    .replace(/[_/\\]+/g, " ");
  if (!blob) return null;
  for (const rule of RULES) {
    if (matchesRule(blob, rule)) return rule.id;
  }
  if (/\bst\d{4,}/.test(blob)) return "seagate";
  if (/\barc [ab]\d/.test(blob)) return "intel";
  return null;
}

function markFill(hex: string): string {
  const h = hex.replace("#", "");
  if (h.length < 6) return `#${h}`;
  const r = parseInt(h.slice(0, 2), 16) / 255;
  const g = parseInt(h.slice(2, 4), 16) / 255;
  const b = parseInt(h.slice(4, 6), 16) / 255;
  const lin = (c: number) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  const luminance = 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
  if (luminance < 0.12) return "#e8eaef";
  return `#${h}`;
}

type BBox = { x: number; y: number; w: number; h: number };

const bboxCache = new Map<string, BBox>();

function pathBBox(d: string): BBox {
  const cached = bboxCache.get(d);
  if (cached) return cached;
  const re = /([MmLlHhVvCcSsQqTtAaZz])|([+-]?(?:\d*\.\d+|\d+)(?:[eE][+-]?\d+)?)/g;
  let m: RegExpExecArray | null;
  let cx = 0;
  let cy = 0;
  let sx = 0;
  let sy = 0;
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  const add = (x: number, y: number) => {
    if (!Number.isFinite(x) || !Number.isFinite(y)) return;
    minX = Math.min(minX, x);
    minY = Math.min(minY, y);
    maxX = Math.max(maxX, x);
    maxY = Math.max(maxY, y);
  };
  const flush = (c: string, a: number[]) => {
    const rel = c === c.toLowerCase();
    const C = c.toUpperCase();
    let i = 0;
    const n = () => a[i++];
    if (C === "M") {
      let first = true;
      while (i + 1 < a.length) {
        cx = rel ? cx + n() : n();
        cy = rel ? cy + n() : n();
        if (first) {
          sx = cx;
          sy = cy;
          first = false;
        }
        add(cx, cy);
      }
      return;
    }
    if (C === "L") {
      while (i + 1 < a.length) {
        cx = rel ? cx + n() : n();
        cy = rel ? cy + n() : n();
        add(cx, cy);
      }
      return;
    }
    if (C === "H") {
      while (i < a.length) {
        cx = rel ? cx + n() : n();
        add(cx, cy);
      }
      return;
    }
    if (C === "V") {
      while (i < a.length) {
        cy = rel ? cy + n() : n();
        add(cx, cy);
      }
      return;
    }
    if (C === "C") {
      while (i + 5 < a.length) {
        const x1 = rel ? cx + n() : n();
        const y1 = rel ? cy + n() : n();
        const x2 = rel ? cx + n() : n();
        const y2 = rel ? cy + n() : n();
        const x = rel ? cx + n() : n();
        const y = rel ? cy + n() : n();
        add(x1, y1);
        add(x2, y2);
        cx = x;
        cy = y;
        add(cx, cy);
      }
      return;
    }
    if (C === "S" || C === "Q") {
      while (i + 3 < a.length) {
        const x1 = rel ? cx + n() : n();
        const y1 = rel ? cy + n() : n();
        const x = rel ? cx + n() : n();
        const y = rel ? cy + n() : n();
        add(x1, y1);
        cx = x;
        cy = y;
        add(cx, cy);
      }
      return;
    }
    if (C === "T") {
      while (i + 1 < a.length) {
        cx = rel ? cx + n() : n();
        cy = rel ? cy + n() : n();
        add(cx, cy);
      }
      return;
    }
    if (C === "A") {
      while (i + 6 < a.length) {
        n();
        n();
        n();
        n();
        n();
        cx = rel ? cx + n() : n();
        cy = rel ? cy + n() : n();
        add(cx, cy);
      }
      return;
    }
    if (C === "Z") {
      cx = sx;
      cy = sy;
    }
  };
  const toks: Array<["c", string] | ["n", number]> = [];
  while ((m = re.exec(d))) {
    if (m[1]) toks.push(["c", m[1]]);
    else toks.push(["n", parseFloat(m[2])]);
  }
  let cur = "M";
  let acc: number[] = [];
  const run = () => {
    if (acc.length || cur.toUpperCase() === "Z") flush(cur, acc);
    acc = [];
  };
  for (const tok of toks) {
    if (tok[0] === "c") {
      run();
      cur = tok[1];
      if (tok[1].toUpperCase() === "Z") run();
    } else acc.push(tok[1]);
  }
  run();
  const box: BBox =
    Number.isFinite(minX) && Number.isFinite(minY)
      ? { x: minX, y: minY, w: Math.max(0.01, maxX - minX), h: Math.max(0.01, maxY - minY) }
      : { x: 0, y: 0, w: 24, h: 24 };
  bboxCache.set(d, box);
  return box;
}

const WORDMARK_ASPECT = 1.55;
const WORDMARK_MAX_WIDTH = 72;

function markLayout(path: string, size: number): { width: number; height: number; viewBox: string } {
  const box = pathBBox(path);
  const aspect = box.w / box.h;
  if (aspect < WORDMARK_ASPECT) {
    return { width: size, height: size, viewBox: "0 0 24 24" };
  }
  const pad = Math.max(box.w, box.h) * 0.04;
  const height = Math.round(Math.min(18, Math.max(14, size * 0.56)));
  const width = Math.min(WORDMARK_MAX_WIDTH, Math.round(height * aspect));
  return {
    width,
    height,
    viewBox: `${box.x - pad} ${box.y - pad} ${box.w + pad * 2} ${box.h + pad * 2}`,
  };
}

export function VendorMark({ vendor, size = 28 }: { vendor: VendorId; size?: number }) {
  const icon = ICONS[vendor];
  if (!icon) return null;
  const layout = markLayout(icon.path, size);
  return (
    <motion.svg
      width={layout.width}
      height={layout.height}
      viewBox={layout.viewBox}
      preserveAspectRatio="xMidYMid meet"
      overflow="visible"
      aria-hidden
      className="shrink-0 self-center"
      fill={markFill(icon.hex)}
      initial={{ opacity: 0, scale: 0.84, rotate: -8 }}
      animate={{ opacity: 1, scale: 1, rotate: 0 }}
      transition={{ duration: 0.4, ease: [0.16, 1, 0.3, 1] }}
    >
      <path d={icon.path} />
    </motion.svg>
  );
}
