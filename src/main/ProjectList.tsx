import { useEffect, useMemo, useRef, type CSSProperties } from "react";
import { CircleAlert, Search, Star } from "lucide-react";
import type { Library, Row } from "../api";
import { openContextMenu } from "../components/menu";
import { colorVar, ProjectIcon } from "../lib/icons";
import { useDevRunning } from "../queries/useDevStatus";
import { isTextInput } from "../commands/registry";
import { focusList } from "../lib/focus";
import { filterKey, useLibrary } from "../state/library";
import { useStore } from "../state/store";
import { projectMenu } from "./projectMenu";

/** Keeps a shown project selected: the one picked last under this filter if shown, else the first. */
export function useSelectionFallback(rows: Row[], settled: boolean) {
  const selectedId = useLibrary((s) => s.selectedId);
  const filter = useLibrary((s) => s.filter);
  const lastSelected = useLibrary((s) => s.lastSelected);
  const select = useLibrary((s) => s.select);
  const setCurrent = useLibrary((s) => s.setCurrent);

  useEffect(() => {
    if (!settled) return;
    const shown = (id: string | null | undefined): id is string => rows.some((r) => r.project.id === id);
    if (shown(selectedId)) return;
    const remembered = lastSelected[filterKey(filter)];
    const next = shown(remembered) ? remembered : (rows[0]?.project.id ?? null);
    if (next !== selectedId) select(next);
  }, [rows, settled, selectedId, filter, lastSelected, select]);

  const current = rows.find((r) => r.project.id === selectedId)?.project ?? null;
  useEffect(() => {
    setCurrent(current);
  }, [current, setCurrent]);
  return current;
}

export function SearchField() {
  const search = useLibrary((s) => s.search);
  const setSearch = useLibrary((s) => s.setSearch);
  const focusRequest = useLibrary((s) => s.searchFocus);
  const field = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (focusRequest === 0) return;
    field.current?.focus();
    field.current?.select();
  }, [focusRequest]);
  return (
    <label className="search-field">
      <Search size={14} aria-hidden />
      <input
        ref={field}
        type="search"
        aria-label="Search Projects"
        placeholder="Search Projects"
        value={search}
        onChange={(e) => setSearch(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Escape" && search) {
            e.stopPropagation();
            setSearch("");
          }
        }}
      />
    </label>
  );
}

function ProjectRow({ row, selected, tags, serverRuns }: { row: Row; selected: boolean; tags: string[]; serverRuns: boolean }) {
  const { project } = row;
  const select = useLibrary((s) => s.select);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (selected) ref.current?.scrollIntoView({ block: "nearest" });
  }, [selected]);
  return (
    <div
      ref={ref}
      role="option"
      aria-selected={selected}
      className={`project-row ${selected ? "project-row--selected" : ""}`}
      style={{ "--row-color": colorVar(project.meta.color) } as CSSProperties}
      onClick={() => select(project.id)}
      onContextMenu={(e) => {
        select(project.id);
        openContextMenu(e, projectMenu(project, tags), project.name);
      }}
    >
      <ProjectIcon icon={project.meta.icon} color={project.meta.color} />
      <span className="project-row__text">
        <span className="project-row__name">{project.name}</span>
        <span className="project-row__subtitle">{row.subtitle}</span>
      </span>
      <span className="project-row__marks">
        {serverRuns && <span className="project-row__running" role="img" aria-label="Dev server running" />}
        {!project.available && <CircleAlert size={14} className="project-row__alert" aria-label="File missing or broken" />}
        {project.meta.starred && <Star size={14} className="project-row__star" aria-label="Starred" />}
      </span>
    </div>
  );
}

/** Something else owns ↑↓: a field, a menu, a popover or a sheet. */
function arrowsTaken(target: EventTarget | null): boolean {
  const { menu, confirmation } = useStore.getState();
  if (menu || confirmation || isTextInput(target)) return true;
  return target instanceof Element && !!target.closest('.popover, [role="menu"], [role="dialog"], [role="alertdialog"]');
}

/** The list's sections; ↑↓ move the selection wherever the keys are, unless something else uses them. */
export function ProjectList({ library, settled }: { library: Library | undefined; settled: boolean }) {
  const sections = useMemo(() => library?.sections ?? [], [library]);
  const rows = useMemo(() => sections.flatMap((s) => s.rows), [sections]);
  const tags = library?.sidebar.tags.map((t) => t.name) ?? [];
  const current = useSelectionFallback(rows, settled);
  const select = useLibrary((s) => s.select);
  const running = new Set(useDevRunning().data);

  useEffect(() => {
    const onKey = (e: globalThis.KeyboardEvent) => {
      if ((e.key !== "ArrowDown" && e.key !== "ArrowUp") || e.defaultPrevented || e.metaKey || e.altKey || e.ctrlKey) return;
      if (arrowsTaken(e.target)) return;
      e.preventDefault();
      const at = rows.findIndex((r) => r.project.id === current?.id);
      const next = rows[Math.max(0, Math.min(rows.length - 1, at + (e.key === "ArrowDown" ? 1 : -1)))];
      if (next) select(next.project.id);
      focusList();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [rows, current?.id, select]);

  return (
    <div className="project-list scroller" role="listbox" aria-label="Projects" tabIndex={0}>
      {library?.emptyText && <p className="project-list__empty">{library.emptyText}</p>}
      {sections.map((section) => (
        <div key={section.title ?? "Projects"} role="group" aria-label={section.title ?? "Projects"}>
          {section.title && <div className="project-list__heading">{section.title}</div>}
          {section.rows.map((row) => (
            <ProjectRow key={row.project.id} row={row} selected={row.project.id === current?.id} tags={tags} serverRuns={running.has(row.project.id)} />
          ))}
        </div>
      ))}
    </div>
  );
}
