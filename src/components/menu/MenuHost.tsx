import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { usePresence } from "../../lib/motion";
import { useStore } from "../../state/store";
import { MenuPanel } from "./MenuPanel";
import type { Placement } from "./placement";
import type { MenuItem, OpenMenu } from "../../state/menuTypes";

/** Shows the menu `openMenu` asked for, once per window; a closed menu fades out. */
export function MenuHost() {
  const menu = useStore((s) => s.menu);
  const close = useStore((s) => s.closeMenu);
  const [shown, setShown] = useState(menu);
  if (menu && menu !== shown) setShown(menu);
  const presence = usePresence(menu !== null);
  if (!presence.mounted || !shown) return null;
  return <OpenMenuLayer key={shown.id} menu={shown} leaving={presence.state === "leaving"} onClose={close} />;
}

function placementOf(at: OpenMenu["at"]): Placement {
  if (!(at instanceof HTMLElement)) return { point: at };
  const { left, top, right, bottom } = at.getBoundingClientRect();
  return { below: { left, top, right, bottom } };
}

interface LayerProps {
  menu: OpenMenu;
  /** Closed, fading out: inert, and no longer listening. */
  leaving: boolean;
  onClose: () => void;
}

/** The open menu and its submenus on the window. Escape, a press outside, a scroll, leaving or resizing the
 *  window, or choosing a row close it; the focus then goes back to where it was. */
function OpenMenuLayer({ menu, leaving, onClose }: LayerProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [placement] = useState(() => placementOf(menu.at));
  const onCloseRef = useRef(onClose);
  useEffect(() => {
    onCloseRef.current = onClose;
  });

  useEffect(() => {
    if (leaving) return;
    const layer = ref.current;
    const inside = (target: EventTarget | null) => target instanceof Node && !!layer?.contains(target);
    const close = () => onCloseRef.current();
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      close();
    };
    // A press on the menu's own button is left to the button, which closes it.
    const onPress = (e: MouseEvent) => {
      const onAnchor = menu.at instanceof HTMLElement && e.target instanceof Node && menu.at.contains(e.target);
      if (!inside(e.target) && !onAnchor) close();
    };
    const onScroll = (e: Event) => {
      if (!inside(e.target)) close();
    };
    window.addEventListener("keydown", onKey, true);
    document.addEventListener("mousedown", onPress, true);
    document.addEventListener("scroll", onScroll, true);
    window.addEventListener("blur", close);
    window.addEventListener("resize", close);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      document.removeEventListener("mousedown", onPress, true);
      document.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("blur", close);
      window.removeEventListener("resize", close);
      // Only when the focus has nowhere else to be: a row may have moved it on (a rename field, a sheet).
      const focused = document.activeElement;
      const lost = !focused || focused === document.body || inside(focused);
      if (lost && menu.opener?.isConnected) menu.opener.focus({ preventScroll: true });
    };
  }, [menu, leaving]);

  const choose = (item: MenuItem) => {
    onClose();
    item.run?.();
  };

  return createPortal(
    <div ref={ref} className={leaving ? "menu-layer fading fading--leaving" : "menu-layer fading"} inert={leaving}>
      <MenuPanel
        entries={menu.entries}
        label={menu.label}
        search={menu.search}
        placement={placement}
        highlightFirst={menu.at instanceof HTMLElement}
        onChoose={choose}
      />
    </div>,
    document.body,
  );
}
