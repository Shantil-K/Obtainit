import type { InstallKind, SourceKind, Status } from "./types";

export function bytes(n: number | null | undefined): string {
  if (!n) return "";
  const u = ["B", "KB", "MB", "GB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < u.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v >= 10 || i === 0 ? 0 : 1)} ${u[i]}`;
}

export function ago(iso: string | null | undefined): string {
  if (!iso) return "";
  const s = (Date.now() - new Date(iso).getTime()) / 1000;
  if (s < 60) return "just now";
  const m = s / 60;
  if (m < 60) return `${Math.floor(m)} min ago`;
  const h = m / 60;
  if (h < 24) return `${Math.floor(h)} h ago`;
  const d = h / 24;
  if (d < 30) return `${Math.floor(d)} day${Math.floor(d) === 1 ? "" : "s"} ago`;
  return new Date(iso).toLocaleDateString();
}

export function date(iso: string | null | undefined): string {
  return iso ? new Date(iso).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" }) : "";
}

export const KIND_LABEL: Record<InstallKind, string> = {
  binary: "Binary",
  archive: "Archive",
  appimage: "AppImage",
  deb: "DEB",
  rpm: "RPM",
  pacman: "Arch pkg",
  flatpak: "Flatpak",
  snap: "Snap",
  installer: "Installer",
  msi: "MSI",
  msix: "MSIX",
  unknown: "Other",
};

export const KIND_ICON: Record<InstallKind, string> = {
  binary: "terminal",
  archive: "folder_zip",
  appimage: "deployed_code",
  deb: "package_2",
  rpm: "package_2",
  pacman: "package_2",
  flatpak: "widgets",
  snap: "widgets",
  installer: "install_desktop",
  msi: "install_desktop",
  msix: "install_desktop",
  unknown: "draft",
};

export const SOURCE_LABEL: Record<SourceKind, string> = {
  github: "GitHub",
  gitlab: "GitLab",
  forgejo: "Codeberg / Forgejo",
  html: "Web page (HTML)",
  json: "JSON API",
};

export const SOURCE_ICON: Record<SourceKind, string> = {
  github: "code",
  gitlab: "code",
  forgejo: "code",
  html: "language",
  json: "data_object",
};

export const STATUS: Record<Status, { label: string; icon: string; tone: string }> = {
  update: { label: "Update available", icon: "upgrade", tone: "primary" },
  installed: { label: "Up to date", icon: "check_circle", tone: "secondary" },
  not_installed: { label: "Not installed", icon: "download", tone: "neutral" },
  track: { label: "Track only", icon: "visibility", tone: "tertiary" },
  error: { label: "Check failed", icon: "error", tone: "error" },
};

export function initials(name: string): string {
  const parts = name.replace(/[-_.]/g, " ").trim().split(/\s+/);
  return ((parts[0]?.[0] ?? "?") + (parts[1]?.[0] ?? "")).toUpperCase();
}
