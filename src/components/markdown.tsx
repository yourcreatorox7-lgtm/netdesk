import type { ReactNode } from "react";
import { toast } from "./ui";

// نمایش‌دهنده‌ی کوچک Markdown برای آموزش‌ها: تیتر، فهرست، کد، نقل‌قول، پررنگ و لینک.
// فقط عنصر React می‌سازد (هیچ innerHTMLی در کار نیست).

export async function copy(text: string, done = "کپی شد") {
  try { await navigator.clipboard.writeText(text); toast(done); } catch { toast("کپی ممکن نشد"); }
}

function inline(text: string): ReactNode[] {
  return text.split(/(`[^`]+`|\*\*[^*]+\*\*|\[[^\]]+\]\([^)\s]+\))/g).map((part, i) => {
    if (part.startsWith("`")) return <code key={i} className="ltr">{part.slice(1, -1)}</code>;
    if (part.startsWith("**")) return <b key={i}>{inline(part.slice(2, -2))}</b>;
    const link = /^\[([^\]]+)\]\(([^)\s]+)\)$/.exec(part);
    if (link) {
      // پیوند بیرونی کپی می‌شود تا کاربر آگاهانه در مرورگر باز کند
      return (
        <button key={i} className="linklike" title={link[2]} onClick={() => copy(link[2], "نشانی کپی شد؛ در مرورگر باز کنید")}>
          {link[1]} <span aria-hidden="true">⧉</span>
        </button>
      );
    }
    return part;
  });
}

export function Markdown({ source }: { source: string }) {
  const lines = source.replace(/\r/g, "").split("\n");
  const out: ReactNode[] = [];
  let i = 0;
  const block = /^(#{1,3} |```|> |- |\d+\. )/;
  while (i < lines.length) {
    const line = lines[i];
    if (!line.trim()) { i++; continue; }
    const key = out.length;
    if (line.startsWith("```")) {
      const code: string[] = [];
      for (i++; i < lines.length && !lines[i].startsWith("```"); i++) code.push(lines[i]);
      i++;
      const text = code.join("\n");
      out.push(
        <div key={key} className="code">
          <button className="copy" onClick={() => copy(text)}>کپی</button>
          <pre className="ltr">{text}</pre>
        </div>,
      );
    } else if (/^#{1,3} /.test(line)) {
      const level = line.indexOf(" ");
      const text = inline(line.slice(level + 1));
      out.push(level === 1 ? <h1 key={key}>{text}</h1> : level === 2 ? <h2 key={key}>{text}</h2> : <h3 key={key}>{text}</h3>);
      i++;
    } else if (line.startsWith("> ")) {
      const quote: string[] = [];
      while (i < lines.length && lines[i].startsWith(">")) quote.push(lines[i++].replace(/^> ?/, ""));
      out.push(<blockquote key={key}>{inline(quote.join(" "))}</blockquote>);
    } else if (/^(- |\d+\. )/.test(line)) {
      const ordered = /^\d/.test(line);
      const items: string[] = [];
      const re = ordered ? /^\d+\. / : /^- /;
      while (i < lines.length && re.test(lines[i])) items.push(lines[i++].replace(re, ""));
      const li = items.map((t, k) => <li key={k}>{inline(t)}</li>);
      out.push(ordered ? <ol key={key}>{li}</ol> : <ul key={key}>{li}</ul>);
    } else {
      const para: string[] = [line];
      for (i++; i < lines.length && lines[i].trim() && !block.test(lines[i]); i++) para.push(lines[i]);
      out.push(<p key={key}>{inline(para.join(" "))}</p>);
    }
  }
  return <div className="md">{out}</div>;
}
