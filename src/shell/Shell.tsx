import type { ReactNode } from "react";
import { PanelLeft } from "lucide-react";
import { DetailTools, ProjectDetail } from "../main/detail/ProjectDetail";
import { Inspector } from "../main/inspector/Inspector";
import { ProjectList, SearchField } from "../main/ProjectList";
import { Sidebar } from "../main/Sidebar";
import { useLibraryView } from "../queries/useLibraryView";
import { useLibrary } from "../state/library";
import { inspectorShown, useStore } from "../state/store";

const classNames = (...names: (string | false)[]) => names.filter(Boolean).join(" ");

/** One pane: a top strip that drags the window, and a body that scrolls. */
function Pane({ as: Element, className, label, strip, children }: {
  as: "nav" | "section" | "main" | "aside";
  className: string;
  label: string;
  strip?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <Element className={`pane ${className}`} aria-label={label}>
      <div className="pane__strip" data-tauri-drag-region>
        {strip}
      </div>
      <div className="pane__body scroller">{children}</div>
    </Element>
  );
}

/** Sidebar, project list, detail and inspector panes; hiding the sidebar also hides the list. */
export function Shell() {
  const sidebarHidden = useStore((s) => s.sidebarHidden);
  const toggleSidebar = useStore((s) => s.toggleSidebar);
  const inspector = useStore(inspectorShown);
  const filter = useLibrary((s) => s.filter);
  const search = useLibrary((s) => s.search);
  const current = useLibrary((s) => s.current);
  const { data: library, isPlaceholderData, isSuccess } = useLibraryView(filter, search);
  const classes = classNames("shell", sidebarHidden && "shell--no-sidebar", inspector && "shell--inspector");
  const sidebarLabel = sidebarHidden ? "Show Sidebar" : "Hide Sidebar";
  return (
    <div className={classes}>
      {!sidebarHidden && (
        <Pane as="nav" className="sidebar" label="Sidebar">
          <Sidebar library={library} />
        </Pane>
      )}
      {/* The list stays mounted while hidden, so its selection keeps the detail filled. */}
      <Pane as="section" className={classNames("list-pane", sidebarHidden && "list-pane--hidden")} label="Projects" strip={<SearchField />}>
        <ProjectList library={library} settled={isSuccess && !isPlaceholderData} />
      </Pane>
      <Pane as="main" className="detail-pane" label="Project" strip={<DetailTools project={current} />}>
        <ProjectDetail project={current} library={library} />
      </Pane>
      {inspector && (
        <Pane as="aside" className="inspector-pane" label="Inspector">
          <Inspector project={current} />
        </Pane>
      )}
      <button type="button" className={classNames("icon-btn sidebar-toggle", !sidebarHidden && "sidebar-toggle--on-dark")} aria-label={sidebarLabel} title={sidebarLabel} onClick={toggleSidebar}>
        <PanelLeft size={16} />
      </button>
    </div>
  );
}
