/** Settings building blocks: groups of rows, label left and control right. */
import { createContext, useContext, useEffect, useId, useRef, type ReactNode } from "react";
import { SegmentedControl, type SegmentedControlProps } from "../components/SegmentedControl";
import { Switch } from "./Switch";

/** The row Settings was opened at, a new object per request, so opening the same row again reveals it again. */
export interface RevealTarget {
  id: string;
  /** Revealed instead while the row isn't shown. */
  under?: string;
}

export const ScrollTarget = createContext<RevealTarget | undefined>(undefined);

/** Targets whose own row was revealed; a fallback revealed first gives way to the row when it mounts later. */
const revealed = new WeakSet<RevealTarget>();

function scrollAndHighlight(element: HTMLElement) {
  element.scrollIntoView?.({ block: "start" });
  element.classList.remove("found");
  void element.offsetWidth; // restarts the highlight
  element.classList.add("found");
}

function useScrollTarget<E extends HTMLElement>(anchor: string | undefined) {
  const target = useContext(ScrollTarget);
  const ref = useRef<E>(null);
  useEffect(() => {
    const element = ref.current;
    if (!anchor || !target || !element || revealed.has(target)) return;
    if (anchor !== target.id && anchor !== target.under) return;
    if (anchor === target.id) revealed.add(target);
    scrollAndHighlight(element);
  }, [anchor, target]);
  return ref;
}

/** A group with its heading and an optional description above it; hairlines between its rows. */
export function Group({ title, description, anchor, children }: { title?: string; description?: ReactNode; anchor?: string; children: ReactNode }) {
  const ref = useScrollTarget<HTMLElement>(anchor);
  return (
    <section ref={ref} className="settings-group" aria-label={title} data-anchor={anchor}>
      {title && <h3>{title}</h3>}
      {description && <p className="settings-group-desc">{description}</p>}
      <div className="settings-card">{children}</div>
    </section>
  );
}

interface RowProps {
  label: string;
  description?: ReactNode;
  /** The row id the page registers. */
  anchor?: string;
  /** The control the label names. */
  htmlFor?: string;
  /** The control goes below the label instead of beside it. */
  stacked?: boolean;
  /** Dimmed: it has no effect with the current settings. */
  disabled?: boolean;
  children?: ReactNode;
}

/** One row: label and description on the left, the control (a value, a `.btn`, a field) on the right. */
export function Row({ label, description, anchor, htmlFor, stacked, disabled, children }: RowProps) {
  const ref = useScrollTarget<HTMLDivElement>(anchor);
  const className = ["settings-row", stacked && "stacked", disabled && "disabled"].filter(Boolean).join(" ");
  return (
    <div ref={ref} className={className} data-anchor={anchor}>
      <div className="settings-row-text">
        {htmlFor ? (
          <label className="settings-row-label" htmlFor={htmlFor}>
            {label}
          </label>
        ) : (
          <div className="settings-row-label">{label}</div>
        )}
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

/** A row whose control is a segmented control. */
export function Segmented<T extends string>({ description, anchor, ...control }: SegmentedControlProps<T> & { description?: ReactNode; anchor?: string }) {
  return (
    <Row label={control.label} description={description} anchor={anchor}>
      <SegmentedControl {...control} />
    </Row>
  );
}

/** A row whose control is a switch, named by the row's label. */
export function SwitchRow({ label, description, anchor, checked, disabled, onChange }: { label: string; description?: ReactNode; anchor?: string; checked: boolean; disabled?: boolean; onChange: (on: boolean) => void }) {
  const id = useId();
  return (
    <Row label={label} description={description} anchor={anchor} htmlFor={id} disabled={disabled}>
      <Switch id={id} checked={checked} disabled={disabled} onChange={onChange} />
    </Row>
  );
}
