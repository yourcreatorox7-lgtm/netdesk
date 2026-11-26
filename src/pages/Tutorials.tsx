import { useEffect, useRef } from "react";
import { Markdown } from "../components/markdown";
import { tutorials, type TutorialGroup } from "../lib/tutorials";

const groups: TutorialGroup[] = ["شروع", "هسته‌ها", "ابزارهای پرخطر"];

export default function Tutorials({ id, open }: { id: string; open: (id: string) => void }) {
  const current = tutorials.find((t) => t.id === id) ?? tutorials[0];
  const article = useRef<HTMLElement>(null);
  useEffect(() => { article.current?.closest("main")?.scrollTo({ top: 0 }); }, [current.id]);

  return (
    <div className="docs">
      <nav className="docs-nav" aria-label="فهرست آموزش‌ها">
        {groups.map((g) => (
          <div key={g}>
            <div className="nav-group">{g}</div>
            {tutorials.filter((t) => t.group === g).map((t) => (
              <button key={t.id} aria-current={t.id === current.id ? "page" : undefined} onClick={() => open(t.id)}>{t.title}</button>
            ))}
          </div>
        ))}
      </nav>
      <article className="panel docs-body" ref={article}>
        <Markdown source={current.body} />
      </article>
    </div>
  );
}
