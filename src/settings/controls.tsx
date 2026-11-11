/** Settings building blocks: inset groups of rows, label left and control right. */
import { createContext, useContext, useEffect, useRef, type ReactNode } from "react";

/** The page or row `open_settings` asked for; the group or row with that anchor scrolls into view. */
export const ScrollTarget = createContext<string | undefined>(undefined);

function useScrollTarget<E extends HTMLElement>(anchor: string | undefined) {
  const target = useContext(ScrollTarget);
  const ref = useRef<E>(null);
  useEffect(() => {
    if (anchor && anchor === target) ref.current?.scrollIntoView?.({ block: "start" });
  }, [anchor, target]);
  return ref;
}

/** An inset group with its heading above it. */
export function Group({ title, anchor, children }: { title: string; anchor?: string; children: ReactNode }) {
  const ref = useScrollTarget<HTMLElement>(anchor);
  return (
    <section ref={ref} className="settings-group" aria-label={title}>
      <h3>{title}</h3>
      <div className="settings-card">{children}</div>
    </section>
  );
}

/** One row: label and description on the left, the control (a value, a `.btn`) on the right. `anchor` is the
 *  row id the page registers. */
export function Row({ label, description, anchor, children }: { label: string; description?: ReactNode; anchor?: string; children?: ReactNode }) {
  const ref = useScrollTarget<HTMLDivElement>(anchor);
  return (
    <div ref={ref} className="settings-row">
      <div className="settings-row-text">
        <div className="settings-row-label">{label}</div>
        {description && <div className="settings-row-desc">{description}</div>}
      </div>
      <div className="settings-row-control">{children}</div>
    </div>
  );
}

/** A read-only value; selectable, and cut off with an ellipsis (the tooltip shows it whole). */
export function Value({ children }: { children: ReactNode }) {
  return <span className="settings-value">{children}</span>;
}
