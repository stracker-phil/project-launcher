import { inspectorShown, useStore } from "../state/store";

/** One pane: a top strip that drags the window, and a body that scrolls. */
function Pane({ as: Element, className, label }: { as: "nav" | "section" | "main" | "aside"; className: string; label: string }) {
  return (
    <Element className={`pane ${className}`} aria-label={label}>
      <div className="pane__strip" data-tauri-drag-region />
      <div className="pane__body scroller" />
    </Element>
  );
}

/** Sidebar, project list, detail and inspector panes; hiding the sidebar also hides the list. */
export function Shell() {
  const sidebarHidden = useStore((s) => s.sidebarHidden);
  const inspector = useStore(inspectorShown);
  const classes = ["shell", sidebarHidden && "shell--no-sidebar", inspector && "shell--inspector"].filter(Boolean).join(" ");
  return (
    <div className={classes}>
      {!sidebarHidden && <Pane as="nav" className="sidebar" label="Sidebar" />}
      {!sidebarHidden && <Pane as="section" className="list-pane" label="Projects" />}
      <Pane as="main" className="detail-pane" label="Project" />
      {inspector && <Pane as="aside" className="inspector-pane" label="Inspector" />}
    </div>
  );
}
