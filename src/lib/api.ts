// لایه‌ی ارتباط با Rust. اگر برنامه بیرون از Tauri (مثلاً در مرورگر برای
// طراحی) اجرا شود، داده‌ی شبیه‌سازی‌شده برمی‌گردد و این را با `isMock` اعلام می‌کند.
import { invoke } from "@tauri-apps/api/core";

export type Locality = "local" | "publicEndpoint" | "requiresAccount";

export interface Summary {
  sent: number; received: number; lossPct: number;
  minMs: number | null; avgMs: number | null; medianMs: number | null;
  p95Ms: number | null; maxMs: number | null; jitterMs: number | null; score: number;
}
export interface ResolverHealth {
  name: string; address: string; summary: Summary; rcode: string | null;
  answers: string[]; warning: string | null; errors: string[]; locality: Locality;
}
export interface TcpResult { target: string; address: string; portOpen: boolean; summary: Summary; samples: (number | null)[]; }
export interface StoredProfile {
  fingerprint: string; protocol: string; name: string; host: string; port: number;
  params: Record<string, string>; folder: string | null; tags: string[]; favorite: boolean;
  standardProxy: boolean; hasSecret: boolean; addedAt: number;
}
export interface Server {
  id: string; name: string; host: string; sshPort: number; sshUser: string;
  sshKeyPath: string | null; rdpPort: number | null; rdpUser: string | null;
  tags: string[]; provider: string | null; notes: string | null;
}
export interface Metrics {
  cpuPct: number | null; cpus: number | null; load: [number, number, number];
  memUsedPct: number; memTotalKb: number; diskUsedPct: number; diskTotalB: number;
  uptimeS: number; health: "Ok" | "Warning" | "Critical";
}
export interface ImportResult {
  wasBase64: boolean; added: number; skippedDuplicates: number; inFileDuplicates: number;
  lines: { line: number; ok: boolean; error?: string; name?: string; duplicate?: boolean; issues?: { severity: string; message: string }[] }[];
}

export type Risk = "medium" | "high" | "critical";
export interface ToolSetup {
  id: string; path: string; args: string; cwd: string | null; sha256: string;
  consentVersion: number | null; consentedAt: number | null;
}
export interface ToolRun { alive: boolean; elevated: boolean; startedAt: number; port: number | null; profile: string | null }
export interface Tool {
  id: string; name: string; url: string; license: string; summary: string; risk: Risk; confirmPhrase: string;
  requiresAdmin: boolean; installsRootCa: boolean; unreviewed: boolean; warnings: string[]; setupHint: string;
  setup: ToolSetup | null; consented: boolean; running: ToolRun | null;
}
export interface Connected {
  fingerprint: string; name: string; port: number; latencyMs: number | null;
  tweak: string; healthy: boolean; since: number;
}
export type AutoPhase = "idle" | "testing" | "connected" | "failed";
export interface AutoStatus { phase: AutoPhase; steps: string[]; connected: Connected | null; systemProxy: boolean }
export interface AutoSettings { systemProxy: boolean; watchdog: boolean; connectOnStart: boolean; updateSubsOnStart: boolean; port: number }
export interface ServerKit { dir: string; fingerprint: string; port: number }
export interface Detected { path: string; args: string; cwd: string | null }
export type AiAction = "none" | "smart_connect" | "disconnect" | "update_subscriptions" | "run_diagnostics" | "open_profiles" | "open_tutorials" | "open_tools";
export interface ChatMessage { role: "user" | "assistant"; content: string }

export const isMock = !("__TAURI_INTERNALS__" in window);

const rnd = (a: number, b: number) => a + Math.random() * (b - a);
const fakeSummary = (base: number): Summary => {
  const avg = rnd(base * 0.8, base * 1.3);
  return { sent: 5, received: 5, lossPct: Math.random() < 0.15 ? 20 : 0, minMs: avg * 0.8, avgMs: avg, medianMs: avg, p95Ms: avg * 1.4, maxMs: avg * 1.5, jitterMs: rnd(1, 9), score: Math.round(rnd(60, 98)) };
};

async function call<T>(cmd: string, args: Record<string, unknown>, mock: () => T): Promise<T> {
  if (isMock) {
    await new Promise((r) => setTimeout(r, rnd(150, 600)));
    return mock();
  }
  return invoke<T>(cmd, args);
}

export const api = {
  dnsCompare: (domain: string, extra: string[] = []) =>
    call<ResolverHealth[]>("dns_compare", { domain, extra }, () =>
      ["Cloudflare|1.1.1.1|18", "Google|8.8.8.8|34", "Quad9|9.9.9.9|46", "Shecan|178.22.122.100|12", "AdGuard|94.140.14.14|71"].map((s) => {
        const [name, ip, b] = s.split("|");
        return { name, address: `${ip}:53`, summary: fakeSummary(+b), rcode: "NoError", answers: ["93.184.215.14"], warning: null, errors: [], locality: "publicEndpoint" as const };
      }).sort((a, b) => (a.summary.avgMs ?? 1e9) - (b.summary.avgMs ?? 1e9))),

  tcpProbe: (target: string, count = 5) =>
    call<TcpResult>("tcp_probe", { target, count }, () => {
      const samples = Array.from({ length: count }, () => (Math.random() < 0.06 ? null : rnd(30, 70)));
      return { target, address: target, portOpen: true, summary: fakeSummary(45), samples };
    }),

  quickReport: (format: "md" | "json" = "md") =>
    call<string>("quick_report", { format }, () => "# گزارش عیب‌یابی شبکه\n\n(داده‌ی نمایشی — در برنامه‌ی دسکتاپ نتیجه‌ی واقعی تولید می‌شود)\n"),

  proxyCheck: (kind: "socks5" | "http", address: string) =>
    call<{ verdict: string; works: boolean }>("proxy_check", { kind, address }, () => ({ verdict: "Works { handshake_ms: 12.4 }", works: true })),

  importProfiles: (text: string, folder?: string) =>
    call<ImportResult>("import_profiles", { text, folder }, () => ({
      wasBase64: false, added: text.split("\n").filter(Boolean).length, skippedDuplicates: 0, inFileDuplicates: 0,
      lines: text.split("\n").filter(Boolean).map((l, i) => ({ line: i + 1, ok: l.includes("://"), name: l.split("#")[1] ?? `خط ${i + 1}`, error: l.includes("://") ? undefined : "قالب ناشناخته", issues: [] })),
    })),
  listProfiles: () => call<StoredProfile[]>("list_profiles", {}, () => demoProfiles),
  updateProfileMeta: (fingerprint: string, patch: Partial<Pick<StoredProfile, "name" | "folder" | "tags" | "favorite">>) =>
    call<void>("update_profile_meta", { fingerprint, ...patch }, () => undefined),
  deleteProfile: (fingerprint: string) => call<void>("delete_profile", { fingerprint }, () => undefined),
  exportProfiles: (fingerprints: string[], redacted: boolean) =>
    call<string>("export_profiles", { fingerprints, redacted }, () => "vless://REDACTED@example.com:443?security=reality#demo"),

  listServers: () => call<Server[]>("list_servers", {}, () => demoServers),
  saveServer: (server: Server) => call<Server>("save_server", { server }, () => ({ ...server, id: server.id || `srv-${Date.now()}` })),
  deleteServer: (id: string) => call<void>("delete_server", { id }, () => undefined),
  serverMetrics: (id: string) =>
    call<Metrics>("server_metrics", { id }, () => ({ cpuPct: rnd(5, 92), cpus: 2, load: [0.4, 0.3, 0.2], memUsedPct: rnd(30, 90), memTotalKb: 2_000_000, diskUsedPct: rnd(20, 70), diskTotalB: 40e9, uptimeS: 86400 * 12, health: "Ok" })),
  rdpLaunch: (id: string) => call<string>("rdp_launch", { id }, () => "mstsc"),

  backupExport: () => call<string>("backup_export", {}, () => JSON.stringify({ format: "netdesk-backup", version: 1, containsSecrets: false }, null, 2)),
  backupImport: (json: string) => call<[number, number]>("backup_import", { json }, () => [0, 0]),

  // ابزارهای خارجی پرخطر — قواعد تأیید در Rust اجرا می‌شود، نه این‌جا
  toolList: () => call<Tool[]>("tool_list", {}, () => demoTools),
  toolConfigure: (id: string, path: string, args: string, cwd: string | null) =>
    call<ToolSetup>("tool_configure", { id, path, args, cwd }, () => ({ id, path, args, cwd, sha256: "0".repeat(64), consentVersion: null, consentedAt: null })),
  toolConsent: (id: string, acknowledged: number[], phrase: string) => call<void>("tool_consent", { id, acknowledged, phrase }, () => undefined),
  toolForget: (id: string) => call<void>("tool_forget", { id }, () => undefined),
  toolStart: (id: string) => call<ToolRun>("tool_start", { id }, () => ({ alive: true, elevated: false, startedAt: Date.now() / 1000, port: null, profile: null })),
  toolStop: (id: string) => call<string>("tool_stop", { id }, () => "متوقف شد"),
  toolLog: (id: string) => call<string>("tool_log", { id }, () => "(لاگ نمایشی)"),
  singboxConnect: (fingerprint: string) => call<Connected>("singbox_connect", { fingerprint }, () => demo.connect(fingerprint)),
  toolDetect: (id: string) => call<Detected | null>("tool_detect", { id }, () => null),

  // اتصال خودکار و اشتراک‌ها
  autoStatus: () => (isMock ? Promise.resolve(demo.status()) : invoke<AutoStatus>("auto_status")),
  netEnv: () => call<string[]>("net_env", {}, () => ["«happ-xray» (VPN/TUN) فعال است و ترافیک NetDesk هم از آن رد می‌شود. برای تست مستقیم، آن را موقتاً خاموش کنید."]),
  smartConnect: () => call<Connected>("smart_connect", {}, () => demo.connect("a57725fad8bc1e25")),
  autoDisconnect: () => call<void>("auto_disconnect", {}, () => demo.disconnect()),
  autoSettingsGet: () => call<AutoSettings>("auto_settings_get", {}, () => ({ ...demo.settings })),
  autoSettingsSet: (settings: AutoSettings) => call<void>("auto_settings_set", { settings }, () => { demo.settings = settings; }),
  subList: () => call<string[]>("sub_list", {}, () => [...demo.subs]),
  subAdd: (url: string) => call<void>("sub_add", { url }, () => { demo.subs.push(url.split("/")[2] ?? url); }),
  subRemove: (index: number) => call<void>("sub_remove", { index }, () => { demo.subs.splice(index, 1); }),
  subUpdate: () => call<{ added: number; errors: string[] }>("sub_update", {}, () => ({ added: 0, errors: [] })),

  // کیت سرور برای سرور خود کاربر
  serverKitCreate: (host: string, port: number, sni: string) =>
    call<ServerKit>("server_kit_create", { host, port, sni }, () => ({ dir: "C:/Users/you/Desktop/NetDesk-Server-Kit-demo", fingerprint: "a57725fad8bc1e25", port })),
  serverKitOpen: (dir: string) => call<void>("server_kit_open", { dir }, () => undefined),

  // دستیار هوش مصنوعی محلی
  aiStatus: () => call<{ running: boolean; models: string[] }>("ai_status", {}, () => ({ running: true, models: ["gemma3:4b"] })),
  aiPull: (model: string) => call<void>("ai_pull", { model }, () => undefined),
  aiPullStatus: () => call<{ running: boolean; ok: boolean | null; line: string }>("ai_pull_status", {}, () => ({ running: false, ok: null, line: "" })),
  aiChat: (model: string, messages: ChatMessage[]) =>
    call<{ reply: string; action: AiAction }>("ai_chat", { model, messages }, () => ({ reply: "(پاسخ نمایشی) می‌توانم بهترین سرور را پیدا کنم و وصل شوم.", action: "smart_connect" })),
  aiReport: () => call<string>("ai_report", {}, () => "(گزارش نمایشی بدون اطلاعات حساس)"),
};

// وضعیت شبیه‌سازی‌شده برای پیش‌نمایش در مرورگر
const demo = {
  conn: null as Connected | null,
  subs: [] as string[],
  settings: { systemProxy: true, watchdog: true, connectOnStart: false, updateSubsOnStart: false, port: 10808 } as AutoSettings,
  connect(fingerprint: string): Connected {
    const name = demoProfiles.find((p) => p.fingerprint === fingerprint)?.name ?? "نمایشی";
    return (this.conn = { fingerprint, name, port: this.settings.port, latencyMs: 214, tweak: "معمولی", healthy: true, since: Math.floor(Date.now() / 1000) });
  },
  disconnect() { this.conn = null; },
  status(): AutoStatus {
    return { phase: this.conn ? "connected" : "idle", steps: this.conn ? ["سنجش دسترسی ۳ پروفایل…", `✓ ${this.conn.name} — 214 ms`] : [], connected: this.conn, systemProxy: !!this.conn && this.settings.systemProxy };
  },
};

const demoTools: Tool[] = [
  { id: "sing-box", name: "sing-box", url: "https://github.com/yourcreatorox7-lgtm/netdesk/blob/main/docs/tutorials/sing-box.fa.md", license: "GPL-3.0-or-later", summary: "هسته‌ی پروکسی برای اتصال واقعی با پروفایل‌ها (نمایشی).", risk: "medium", confirmPhrase: "می‌دانم چه می‌کنم", requiresAdmin: false, installsRootCa: false, unreviewed: false, warnings: ["هشدار نمایشی ۱", "هشدار نمایشی ۲"], setupHint: "مسیر sing-box.exe", setup: null, consented: false, running: null },
  { id: "master-http-relay", name: "MasterHttpRelayVPN", url: "https://github.com/yourcreatorox7-lgtm/netdesk/blob/main/docs/tutorials/master-http-relay.fa.md", license: "MIT", summary: "پروکسی محلی با رهگیری TLS (نمایشی).", risk: "critical", confirmPhrase: "مسئولیت کامل را می‌پذیرم", requiresAdmin: false, installsRootCa: true, unreviewed: false, warnings: ["گواهی CA ریشه نصب می‌کند", "حساب Google ممکن است تعلیق شود"], setupHint: "مسیر python.exe", setup: null, consented: false, running: null },
];

const demoProfiles: StoredProfile[] = [
  { fingerprint: "a57725fad8bc1e25", protocol: "vless", name: "آلمان — Reality", host: "de1.example.net", port: 443, params: { security: "reality", type: "tcp" }, folder: "شخصی", tags: ["سریع"], favorite: true, standardProxy: false, hasSecret: true, addedAt: 0 },
  { fingerprint: "ff05c29743f6c765", protocol: "ss", name: "هلند", host: "nl.example.org", port: 8388, params: { method: "chacha20-ietf-poly1305" }, folder: "شخصی", tags: [], favorite: false, standardProxy: false, hasSecret: true, addedAt: 0 },
  { fingerprint: "0c1d2e3f40516273", protocol: "socks5", name: "پروکسی شبکه‌ی محل کار", host: "10.0.0.2", port: 1080, params: {}, folder: "کار", tags: ["LAN"], favorite: false, standardProxy: true, hasSecret: false, addedAt: 0 },
];
const demoServers: Server[] = [
  { id: "srv-1", name: "VPS فرانکفورت", host: "203.0.113.10", sshPort: 22, sshUser: "deploy", sshKeyPath: "~/.ssh/id_ed25519", rdpPort: null, rdpUser: null, tags: ["prod"], provider: "Hetzner", notes: null },
  { id: "srv-2", name: "ویندوز سرور دفتر", host: "198.51.100.7", sshPort: 22, sshUser: "admin", sshKeyPath: null, rdpPort: 3389, rdpUser: "Administrator", tags: ["RDP"], provider: null, notes: null },
];
