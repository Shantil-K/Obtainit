// Theming. Three layers:
//  1. Default themes: a Material 3 scheme generated from the palette (seed)
//     color, as Flutter's ColorScheme.fromSeed does in Autonomix/Obtainium.
//  2. An optional accent color that replaces the primary color roles.
//  3. Custom themes (templates or user-made): background, surface, accent
//     and text colors, expanded into the full set of M3 color roles.
// Everything ends up in the --md-sys-color-* tokens @material/web reads.

import {
  Hct,
  MaterialDynamicColors,
  SchemeTonalSpot,
  TonalPalette,
  argbFromHex,
  hexFromArgb,
} from "@material/material-color-utilities";
import type { CustomTheme, Settings } from "./types";

const ROLES = [
  "primary", "onPrimary", "primaryContainer", "onPrimaryContainer", "inversePrimary",
  "secondary", "onSecondary", "secondaryContainer", "onSecondaryContainer",
  "tertiary", "onTertiary", "tertiaryContainer", "onTertiaryContainer",
  "error", "onError", "errorContainer", "onErrorContainer",
  "background", "onBackground", "surface", "onSurface", "surfaceVariant", "onSurfaceVariant",
  "surfaceDim", "surfaceBright", "surfaceContainerLowest", "surfaceContainerLow",
  "surfaceContainer", "surfaceContainerHigh", "surfaceContainerHighest",
  "inverseSurface", "inverseOnSurface", "outline", "outlineVariant", "shadow", "scrim", "surfaceTint",
] as const;

type Vars = Record<string, string>;

const kebab = (s: string) => s.replace(/[A-Z]/g, (m) => "-" + m.toLowerCase());

/** Palette (seed) color swatches. */
export const SEEDS = ["#6750A4", "#0B57D0", "#00696B", "#386A20", "#8B5000", "#B3261E", "#984061", "#5C5F61"];
/** Accent swatches: brighter, more saturated picks. */
export const ACCENTS = ["#7C4DFF", "#2979FF", "#00B8D4", "#00C853", "#FFAB00", "#FF6D00", "#FF1744", "#F50057", "#D500F9"];

/** Ready-made themes. `accent: ""` follows the app's accent/palette color. */
export const PRESETS: CustomTheme[] = [
  { id: "preset:paper", name: "Paper", dark: false, background: "#FBF8F3", surface: "#F2EDE4", accent: "#8B5000", text: "#1F1B16" },
  { id: "preset:solarized-light", name: "Solarized Light", dark: false, background: "#FDF6E3", surface: "#EEE8D5", accent: "#268BD2", text: "#073642" },
  { id: "preset:nord-light", name: "Nord Light", dark: false, background: "#ECEFF4", surface: "#E5E9F0", accent: "#5E81AC", text: "#2E3440" },
  { id: "preset:dawn", name: "Rosé Dawn", dark: false, background: "#FAF4ED", surface: "#F2E9E1", accent: "#907AA9", text: "#575279" },
  { id: "preset:black", name: "Black (OLED)", dark: true, background: "#000000", surface: "#101012", accent: "", text: "#E8E6EA" },
  { id: "preset:nord", name: "Nord", dark: true, background: "#2E3440", surface: "#3B4252", accent: "#88C0D0", text: "#ECEFF4" },
  { id: "preset:dracula", name: "Dracula", dark: true, background: "#282A36", surface: "#343746", accent: "#BD93F9", text: "#F8F8F2" },
  { id: "preset:solarized-dark", name: "Solarized Dark", dark: true, background: "#002B36", surface: "#073642", accent: "#2AA198", text: "#EEE8D5" },
  { id: "preset:mocha", name: "Catppuccin Mocha", dark: true, background: "#1E1E2E", surface: "#313244", accent: "#CBA6F7", text: "#CDD6F4" },
  { id: "preset:midnight", name: "Midnight", dark: true, background: "#0F1420", surface: "#182033", accent: "#7AA2F7", text: "#E6E9F2" },
];

// ---------- color math ----------

export function isHex(s: string): boolean {
  return /^#[0-9a-f]{6}$/i.test(s);
}

function rgb(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function toHex([r, g, b]: number[]): string {
  return "#" + [r, g, b].map((v) => Math.round(Math.max(0, Math.min(255, v))).toString(16).padStart(2, "0")).join("");
}

/** Blend `a` toward `b` by `w` (0..1). */
export function mix(a: string, b: string, w: number): string {
  const x = rgb(a), y = rgb(b);
  return toHex(x.map((v, i) => v + (y[i] - v) * w));
}

function luminance(hex: string): number {
  const [r, g, b] = rgb(hex).map((v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG contrast ratio (1..21). */
export function contrast(a: string, b: string): number {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

/** Black or white, whichever reads better on `bg`. */
function onColor(bg: string): string {
  return contrast(bg, "#000000") >= contrast(bg, "#ffffff") ? "#000000" : "#ffffff";
}

// ---------- scheme building ----------

function safe(hex: string, fallback: string) {
  return isHex(hex) ? hex : fallback;
}

function baseScheme(seed: string, dark: boolean): Vars {
  const scheme = new SchemeTonalSpot(Hct.fromInt(argbFromHex(safe(seed, SEEDS[0]))), dark, 0);
  const dyn = MaterialDynamicColors as unknown as Record<string, { getArgb(s: SchemeTonalSpot): number }>;
  const out: Vars = {};
  for (const role of ROLES) {
    const c = dyn[role];
    if (c) out[role] = hexFromArgb(c.getArgb(scheme));
  }
  return out;
}

/** Replace the primary roles with tones of `accent`. */
function withAccent(v: Vars, accent: string, dark: boolean): Vars {
  if (!isHex(accent)) return v;
  const p = TonalPalette.fromInt(argbFromHex(accent));
  const t = (n: number) => hexFromArgb(p.tone(n));
  return {
    ...v,
    primary: t(dark ? 80 : 40),
    onPrimary: t(dark ? 20 : 100),
    primaryContainer: t(dark ? 30 : 90),
    onPrimaryContainer: t(dark ? 90 : 10),
    inversePrimary: t(dark ? 40 : 80),
    surfaceTint: t(dark ? 80 : 40),
  };
}

/** Expand a custom theme's four colors into every M3 role. */
export function customVars(theme: CustomTheme, appAccent: string): Vars {
  const bg = safe(theme.background, theme.dark ? "#141218" : "#fef7ff");
  const surface = safe(theme.surface, bg);
  const text = safe(theme.text, theme.dark ? "#e6e0e9" : "#1d1b20");
  const accent = safe(theme.accent, appAccent);
  // Secondary/tertiary/error come from a generated scheme so they harmonize.
  const v = baseScheme(accent, theme.dark);
  const step = (w: number) => mix(surface, text, w);
  const onAccent = onColor(accent);
  const primaryContainer = mix(accent, bg, theme.dark ? 0.6 : 0.72);
  return {
    ...v,
    primary: accent,
    onPrimary: onAccent,
    primaryContainer,
    onPrimaryContainer: contrast(primaryContainer, text) >= 4.5 ? text : onColor(primaryContainer),
    inversePrimary: mix(accent, theme.dark ? "#000000" : "#ffffff", 0.35),
    surfaceTint: accent,
    secondaryContainer: mix(accent, bg, theme.dark ? 0.72 : 0.82),
    onSecondaryContainer: text,
    background: bg,
    onBackground: text,
    surface: bg,
    surfaceDim: mix(bg, "#000000", 0.06),
    surfaceBright: mix(bg, text, 0.06),
    onSurface: text,
    surfaceVariant: step(0.1),
    onSurfaceVariant: mix(text, bg, 0.28),
    surfaceContainerLowest: mix(bg, surface, 0.4),
    surfaceContainerLow: surface,
    surfaceContainer: step(0.04),
    surfaceContainerHigh: step(0.08),
    surfaceContainerHighest: step(0.12),
    outline: mix(text, bg, 0.5),
    outlineVariant: mix(text, bg, 0.78),
    inverseSurface: text,
    inverseOnSurface: bg,
  };
}

export function allThemes(s: Settings): CustomTheme[] {
  return [...PRESETS, ...(s.custom_themes ?? [])];
}

export function isDarkMode(mode: Settings["theme"]): boolean {
  return mode === "dark" || (mode === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
}

/** The palette color used as accent when none is set. */
export function effectiveAccent(s: Settings): string {
  return isHex(s.accent_color) ? s.accent_color : safe(s.seed_color, SEEDS[0]);
}

/** Every color variable for the current settings. */
export function computeVars(s: Settings, override?: CustomTheme): { vars: Vars; dark: boolean } {
  if (override) return { vars: customVars(override, effectiveAccent(s)), dark: override.dark };
  const dark = isDarkMode(s.theme);
  // Older settings: "pure black" = the Black (OLED) template.
  const id = dark ? s.dark_theme || (s.pure_black ? "preset:black" : "") : s.light_theme;
  const theme = id ? allThemes(s).find((t) => t.id === id && t.dark === dark) : undefined;
  if (theme) return { vars: customVars(theme, effectiveAccent(s)), dark };
  return { vars: withAccent(baseScheme(s.seed_color, dark), s.accent_color, dark), dark };
}

/** Turn role names into CSS custom properties, e.g. for a scoped preview. */
export function cssVars(vars: Vars): string {
  return Object.entries(vars)
    .map(([k, v]) => `--md-sys-color-${kebab(k)}:${v}`)
    .join(";");
}

export function applyTheme(s: Settings, override?: CustomTheme) {
  const { vars, dark } = computeVars(s, override);
  const root = document.documentElement;
  for (const [k, v] of Object.entries(vars)) root.style.setProperty(`--md-sys-color-${kebab(k)}`, v);
  root.dataset.theme = dark ? "dark" : "light";
  root.style.colorScheme = dark ? "dark" : "light";
}
