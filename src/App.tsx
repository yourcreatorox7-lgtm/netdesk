import { lazy, Suspense, useEffect, useState } from "react";
import { Toaster } from "./components/ui";
import { useAuto } from "./lib/useAuto";
import Home from "./pages/Home";

const Dashboard = lazy(() => import("./pages/Dashboard"));
const Diagnostics = lazy(() => import("./pages/Diagnostics"));
const Analyzer = lazy(() => import("./pages/Analyzer"));
const Profiles = lazy(() => import("./pages/Profiles"));
const Servers = lazy(() => import("./pages/Servers"));
const Settings = lazy(() => import("./pages/Settings"));
const Tools = lazy(() => import("./pages/Tools"));
const Assistant = lazy(() => import("./pages/Assistant"));
const MyServer = lazy(() => import("./pages/MyServer"));
const Tutorials = lazy(() => import("./pages/Tutorials"));

export type Route = "home" | "myserver" | "profiles" | "assistant" | "tutorials" | "dashboard" | "diagnostics" | "analyzer" | "servers" | "tools" | "settings";

/** مسیر آیکن‌های خطی ۲۴×۲۴ */
const icons: Record<Route, string> = {
  home: "M4 11 12 4l8 7M6 10v10h12V10",
  myserver: "M4 4h16v6H4zM4 14h16v6H4zM7 7h.01M7 17h.01M12 7h5M12 17h5",
  profiles: "M5 5h14v5H5zM5 14h14v5H5zM8 7.5h.01M8 16.5h.01",
  assistant: "M4 5h16v11H9l-5 4zM9 10.5h.01M12 10.5h.01M15 10.5h.01",
  tutorials: "M12 6C9 4 6 4 4 5v14c2-1 5-1 8 1 3-2 6-2 8-1V5c-2-1-5-1-8 1zm0 0v14",
  dashboard: "M3 13h4l3-7 4 13 3-9 1 3h3",
  diagnostics: "M10 4a6 6 0 1 0 0 12 6 6 0 0 0 0-12zm4.5 10.5L20 20",
  analyzer: "M5 20V10m7 10V4m7 16v-7",
  servers: "M4 5h16v10H4zM9 20h6m-3-5v5",
  tools: "M12 3 2.5 20h19zM12 10v4m0 3h.01",
  settings: "M4 7h10m4 0h2M4 17h4m4 0h8M16 5v4M10 15v4",
};

const NAV: { group: string; items: { id: Route; label: string; key?: string }[] }[] = [
  { group: "اصلی", items: [
    { id: "home", label: "خانه", key: "1" },
    { id: "myserver", label: "سرور خودم" },
    { id: "profiles", label: "پروفایل‌ها", key: "2" },
    { id: "assistant", label: "دستیار هوشمند", key: "3" },
    { id: "tutorials", label: "آموزش‌ها", key: "4" },
  ] },
  { group: "ابزارهای شبکه", items: [
    { id: "dashboard", label: "وضعیت شبکه", key: "5" },
    { id: "diagnostics", label: "عیب‌یابی", key: "6" },
    { id: "analyzer", label: "تحلیلگر و گزارش", key: "7" },
    { id: "servers", label: "VPS و RDP", key: "8" },
  ] },
  { group: "پیشرفته", items: [
    { id: "tools", label: "هسته و ابزارها" },
    { id: "settings", label: "تنظیمات" },
  ] },
];

type Theme = "dark" | "light" | "system";
const readTheme = (): Theme => {
  try { return (localStorage.getItem("theme") as Theme) || "system"; } catch { return "system"; }
};

export default function App() {
  const [route, setRoute] = useState<Route>("home");
  const [doc, setDoc] = useState("start");
  const [theme, setTheme] = useState<Theme>(readTheme);
  const { status } = useAuto();

  const openTutorial = (id: string) => { setDoc(id); setRoute("tutorials"); };

  useEffect(() => {
    const mq = matchMedia("(prefers-color-scheme: light)");
    const apply = () => (document.documentElement.dataset.theme = theme === "system" ? (mq.matches ? "light" : "dark") : theme);
    apply();
    mq.addEventListener("change", apply);
    try { localStorage.setItem("theme", theme); } catch { /* حالت خصوصی */ }
    return () => mq.removeEventListener("change", apply);
  }, [theme]);

  // میان‌برهای صفحه‌کلید
  useEffect(() => {
    const flat = NAV.flatMap((g) => g.items);
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      const hit = flat.find((i) => i.key === e.key);
      if (hit) { e.preventDefault(); setRoute(hit.id); }
      else if (e.key.toLowerCase() === "i") { e.preventDefault(); setRoute("profiles"); }
      else if (e.key.toLowerCase() === "r" && !e.shiftKey) { e.preventDefault(); setRoute("analyzer"); }
      else if (e.shiftKey && e.key.toLowerCase() === "l") { e.preventDefault(); setTheme((t) => (t === "light" ? "dark" : "light")); }
    };
    addEventListener("keydown", onKey);
    return () => removeEventListener("keydown", onKey);
  }, []);

  const page = {
    home: <Home go={setRoute} tutorial={openTutorial} />,
    myserver: <MyServer go={setRoute} tutorial={openTutorial} />, profiles: <Profiles />, assistant: <Assistant go={setRoute} />, tutorials: <Tutorials id={doc} open={setDoc} />,
    dashboard: <Dashboard go={setRoute} />, diagnostics: <Diagnostics />, analyzer: <Analyzer />, servers: <Servers />,
    tools: <Tools tutorial={openTutorial} />, settings: <Settings theme={theme} setTheme={setTheme} />,
  }[route];

  const conn = status.connected;
  const pill = status.phase === "testing" ? { cls: "warn", text: "در حال اتصال…" }
    : conn ? { cls: conn.healthy ? "ok live" : "warn", text: conn.healthy ? "متصل" : "اتصال ضعیف" }
    : { cls: "off", text: "قطع" };

  return (
    <div className="shell">
      <aside className="side">
        <div className="brand">
          <svg width="28" height="28" viewBox="0 0 26 26" aria-hidden="true">
            <rect width="26" height="26" rx="7" fill="var(--panel-2)" />
            <path d="M3 15h4l2-6 3 10 3-13 2 9h6" fill="none" stroke="var(--signal)" strokeWidth="2" strokeLinejoin="round" />
          </svg>
          NetDesk
        </div>
        <button className="status-pill" onClick={() => setRoute("home")} title="رفتن به خانه">
          <span className={`dot ${pill.cls}`} />
          <span>{pill.text}</span>
          {conn && <span className="muted name">{conn.name}</span>}
        </button>
        <nav className="nav" aria-label="بخش‌ها">
          {NAV.map((g) => (
            <div key={g.group}>
              <div className="nav-group">{g.group}</div>
              {g.items.map((i) => (
                <a key={i.id} href={`#${i.id}`} aria-current={route === i.id ? "page" : undefined} onClick={(e) => { e.preventDefault(); setRoute(i.id); }}>
                  <svg className="icon" viewBox="0 0 24 24" aria-hidden="true"><path d={icons[i.id]} /></svg>
                  {i.label}{i.key && <kbd className="ltr">Ctrl {i.key}</kbd>}
                </a>
              ))}
            </div>
          ))}
        </nav>
        <div className="side-foot">نسخه‌ی ۰٫۲، متن‌باز (MIT)</div>
      </aside>
      <main>
        <div className="page" key={route}>
          <Suspense fallback={<p className="muted">بارگذاری…</p>}>{page}</Suspense>
        </div>
      </main>
      <Toaster />
    </div>
  );
}
