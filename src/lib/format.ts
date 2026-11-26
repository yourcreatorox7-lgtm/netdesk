// قالب‌بندی اعداد با ارقام فارسی
const fa = new Intl.NumberFormat("fa-IR", { maximumFractionDigits: 1 });
export const num = (v: number | null | undefined, digits = 1) =>
  v == null || Number.isNaN(v) ? "—" : new Intl.NumberFormat("fa-IR", { maximumFractionDigits: digits }).format(v);
export const ms = (v: number | null | undefined) => (v == null ? "—" : `${fa.format(v)} ms`);
export const pct = (v: number | null | undefined) => (v == null ? "—" : `${fa.format(v)}٪`);
export const bytes = (b: number) => {
  const u = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  while (b >= 1024 && i < u.length - 1) { b /= 1024; i++; }
  return `${fa.format(b)} ${u[i]}`;
};
export const uptime = (s: number) => {
  const d = Math.floor(s / 86400), h = Math.floor((s % 86400) / 3600);
  return d ? (h ? `${num(d, 0)} روز و ${num(h, 0)} ساعت` : `${num(d, 0)} روز`) : `${num(h, 0)} ساعت`;
};
export const level = (p: number | null | undefined) => (p == null ? "" : p > 90 ? "bad" : p > 75 ? "warn" : "");
