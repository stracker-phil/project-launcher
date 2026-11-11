import { useEffect, useLayoutEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import { createPortal } from "react-dom";
import { restoreFocus } from "../lib/focus";
import { usePresence } from "../lib/motion";

const FOCUSABLE = 'button:not(:disabled), input:not(:disabled), textarea:not(:disabled), select:not(:disabled), [tabindex]:not([tabindex="-1"])';

const EDGE = 6;
const GAP = 4;

interface Props {
  open: boolean;
  anchor: RefObject<HTMLElement | null>;
  label: string;
  onClose: () => void;
  children: ReactNode;
}

/** A panel under its anchor, kept inside the window; Esc or a press outside closes it and the focus goes back. */
export function Popover({ open, anchor, label, onClose, children }: Props) {
  const { mounted, state } = usePresence(open);
  const panel = useRef<HTMLDivElement>(null);
  const [place, setPlace] = useState({ left: 0, top: 0 });

  useLayoutEffect(() => {
    if (!open || !anchor.current || !panel.current) return;
    const at = anchor.current.getBoundingClientRect();
    const width = panel.current.offsetWidth;
    setPlace({
      left: Math.max(EDGE, Math.min(at.left, window.innerWidth - width - EDGE)),
      top: at.bottom + GAP,
    });
  }, [open, anchor, mounted]);

  // The panel takes the keys (its pressed choice first); closing gives them back to the anchor.
  useEffect(() => {
    if (!open) return;
    const opener = anchor.current;
    let root = panel.current;
    const focusInPanel = requestAnimationFrame(() => {
      root ??= panel.current;
      if (!root || root.contains(document.activeElement)) return;
      (root.querySelector<HTMLElement>('[aria-pressed="true"]') ?? root.querySelector<HTMLElement>(FOCUSABLE))?.focus();
    });
    return () => {
      cancelAnimationFrame(focusInPanel);
      const active = document.activeElement;
      if (active && active !== document.body && !root?.contains(active)) return;
      if (opener?.isConnected && opener.tabIndex >= 0) opener.focus();
      else restoreFocus();
    };
  }, [open, anchor]);

  useEffect(() => {
    if (!open) return;
    const opener = anchor.current;
    const onPress = (e: MouseEvent) => {
      const target = e.target as Node;
      if (!panel.current?.contains(target) && !opener?.contains(target)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      // Escape while a shortcut field records only cancels the recording.
      if (e.target instanceof Element && e.target.closest("[data-recording]")) return;
      e.preventDefault();
      e.stopPropagation();
      onClose();
      opener?.focus();
    };
    document.addEventListener("mousedown", onPress);
    document.addEventListener("keydown", onKey, true);
    return () => {
      document.removeEventListener("mousedown", onPress);
      document.removeEventListener("keydown", onKey, true);
    };
  }, [open, anchor, onClose]);

  if (!mounted) return null;
  return createPortal(
    <div className={`fading ${state === "leaving" ? "fading--leaving" : ""}`} inert={state === "leaving" || undefined}>
      <div ref={panel} className="popover" role="dialog" aria-label={label} style={place}>
        {children}
      </div>
    </div>,
    document.body,
  );
}
