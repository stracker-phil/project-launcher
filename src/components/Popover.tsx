import { useEffect, useLayoutEffect, useRef, useState, type ReactNode, type RefObject } from "react";
import { createPortal } from "react-dom";
import { usePresence } from "../lib/motion";

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
