import { useRef, useState, type ReactNode } from "react";
import { ChevronDown } from "lucide-react";
import { useHeightGlide } from "../../lib/motion";

const storageKey = (title: string) => `inspector.collapsed.${title}`;

function readCollapsed(title: string): boolean {
  try {
    return localStorage.getItem(storageKey(title)) === "1";
  } catch {
    return false;
  }
}

function storeCollapsed(title: string, collapsed: boolean) {
  try {
    if (collapsed) localStorage.setItem(storageKey(title), "1");
    else localStorage.removeItem(storageKey(title));
  } catch {
    // Folding still works for this session.
  }
}

/** A titled inspector section that folds; the fold is remembered per title in this browser. */
export function Section({ title, children }: { title: string; children: ReactNode }) {
  const [collapsed, setCollapsed] = useState(() => readCollapsed(title));
  const glide = useRef<HTMLDivElement>(null);
  useHeightGlide(glide, collapsed);
  const toggle = () => {
    const next = !collapsed;
    storeCollapsed(title, next);
    setCollapsed(next);
  };
  return (
    <div ref={glide}>
      <section className="inspector-section" aria-label={title}>
        <h3 className="inspector-section__head">
          <button type="button" className="inspector-section__toggle" aria-expanded={!collapsed} onClick={toggle}>
            <span className="inspector-section__title">{title}</span>
            <ChevronDown size={14} className="inspector-section__chevron" aria-hidden />
          </button>
        </h3>
        {!collapsed && <div className="inspector-section__body">{children}</div>}
      </section>
    </div>
  );
}
