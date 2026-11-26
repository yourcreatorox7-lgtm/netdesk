// آموزش‌ها: همان فایل‌های docs/tutorials که در مخزن هم دیده می‌شوند (یک منبع واحد)
import start from "../../docs/tutorials/start.fa.md?raw";
import singBox from "../../docs/tutorials/sing-box.fa.md?raw";
import myServer from "../../docs/tutorials/my-server.fa.md?raw";
import ai from "../../docs/tutorials/ai.fa.md?raw";
import xray from "../../docs/tutorials/xray.fa.md?raw";
import masterHttpRelay from "../../docs/tutorials/master-http-relay.fa.md?raw";
import mhrCfw from "../../docs/tutorials/mhr-cfw.fa.md?raw";
import flowdriver from "../../docs/tutorials/flowdriver.fa.md?raw";
import sniPy from "../../docs/tutorials/sni-spoofing-py.fa.md?raw";
import sniRust from "../../docs/tutorials/sni-spoofing-rust.fa.md?raw";
import sniGo from "../../docs/tutorials/sni-spoofing-go.fa.md?raw";

export type TutorialGroup = "شروع" | "هسته‌ها" | "ابزارهای پرخطر";
export interface Tutorial { id: string; title: string; group: TutorialGroup; body: string }

export const tutorials: Tutorial[] = [
  { id: "start", title: "از صفر تا اتصال", group: "شروع", body: start },
  { id: "sing-box", title: "نصب sing-box", group: "شروع", body: singBox },
  { id: "my-server", title: "ساخت سرور خودم", group: "شروع", body: myServer },
  { id: "ai", title: "دستیار هوش مصنوعی", group: "شروع", body: ai },
  { id: "xray", title: "Xray-core", group: "هسته‌ها", body: xray },
  { id: "master-http-relay", title: "MasterHttpRelayVPN", group: "ابزارهای پرخطر", body: masterHttpRelay },
  { id: "mhr-cfw", title: "mhr-cfw", group: "ابزارهای پرخطر", body: mhrCfw },
  { id: "flowdriver", title: "FlowDriver", group: "ابزارهای پرخطر", body: flowdriver },
  { id: "sni-spoofing-py", title: "SNI-Spoofing (Python)", group: "ابزارهای پرخطر", body: sniPy },
  { id: "sni-spoofing-rust", title: "sni-spoofing-rust", group: "ابزارهای پرخطر", body: sniRust },
  { id: "sni-spoofing-go", title: "SNI-Spoofing-Go", group: "ابزارهای پرخطر", body: sniGo },
];
