import { useEffect, useRef, useState, type ReactNode } from "react";
import { Archive, FolderClosed, Plus, Settings, Star, Tag, Tags } from "lucide-react";
import { api, type Filter, type Library, type TagCount } from "../api";
import { openContextMenu } from "../components/menu";
import { attempt } from "../state/attempt";
import { sameFilter, useLibrary, type TagEdit } from "../state/library";
import { useStore } from "../state/store";
import { restoreFocus } from "../lib/focus";
import { addProjects } from "./actions";

function FilterRow({ filter, icon, title, count, onContextMenu, onDoubleClick }: {
  filter: Filter;
  icon: ReactNode;
  title: string;
  count: number;
  onContextMenu?: (e: React.MouseEvent) => void;
  onDoubleClick?: () => void;
}) {
  const current = useLibrary((s) => s.filter);
  const setFilter = useLibrary((s) => s.setFilter);
  const selected = sameFilter(current, filter);
  return (
    <button
      type="button"
      className={`sidebar-row ${selected ? "sidebar-row--selected" : ""}`}
      aria-current={selected || undefined}
      aria-label={count > 0 ? `${title} ${count}` : title}
      onClick={() => setFilter(filter)}
      onContextMenu={onContextMenu}
      onDoubleClick={onDoubleClick}
    >
      <span className="sidebar-row__icon" aria-hidden>{icon}</span>
      <span className="sidebar-row__title">{title}</span>
      <span className="sidebar-row__count">{count > 0 ? count : ""}</span>
    </button>
  );
}

/** The new or renamed tag's field: Enter or leaving saves, Esc or an unchanged name cancels. */
function TagField({ initial, onDone }: { initial: string; onDone: (name: string | null) => void }) {
  const [name, setName] = useState(initial);
  const field = useRef<HTMLInputElement>(null);
  const done = useRef(false);
  useEffect(() => {
    field.current?.focus();
    field.current?.select();
  }, []);
  const finish = (value: string | null) => {
    if (done.current) return;
    done.current = true;
    const trimmed = value?.trim() ?? "";
    onDone(trimmed && trimmed !== initial ? trimmed : null);
    restoreFocus();
  };
  return (
    <div className="sidebar-row sidebar-row--editing">
      <span className="sidebar-row__icon" aria-hidden><Tag size={16} /></span>
      <input
        ref={field}
        className="sidebar-row__field"
        aria-label="Tag name"
        placeholder="Tag name"
        value={name}
        onChange={(e) => setName(e.target.value)}
        onBlur={() => document.hasFocus() && finish(name)}
        onKeyDown={(e) => {
          if (e.nativeEvent.isComposing) return;
          if (e.key === "Enter") finish(name);
          if (e.key === "Escape") {
            e.stopPropagation();
            finish(null);
          }
        }}
      />
    </div>
  );
}

function Group({ title, add, children }: { title?: string; add?: { label: string; run: () => void }; children: ReactNode }) {
  return (
    <div className="sidebar-group" role="group" aria-label={title}>
      {title && (
        <div className="sidebar-group__head">
          <span>{title}</span>
          {add && (
            <button type="button" className="icon-btn tiny sidebar-group__add" title={add.label} aria-label={add.label} onClick={add.run}>
              <Plus size={14} />
            </button>
          )}
        </div>
      )}
      {children}
    </div>
  );
}

async function deleteTag(name: string, count: number) {
  const confirmed = await useStore.getState().confirm({
    title: `Delete tag “${name}”?`,
    text: count === 0 ? "No projects use this tag." : `${count} ${count === 1 ? "project becomes" : "projects become"} untagged.`,
    action: "Delete",
  });
  if (!confirmed) return;
  const library = useLibrary.getState();
  if (await attempt(api.deleteTag(name), "Couldn’t delete tag")) {
    if (sameFilter(library.filter, { kind: "tag", tag: name })) library.setFilter({ kind: "all" });
  }
}

function addTag(name: string) {
  return attempt(api.addTag(name), "Couldn’t create tag");
}

/** Renaming onto an existing tag merges the two (Rust's rule); a selected tag's filter follows. */
async function renameTag(from: string, to: string) {
  const library = useLibrary.getState();
  if (await attempt(api.renameTag(from, to), "Couldn’t rename tag")) {
    if (sameFilter(library.filter, { kind: "tag", tag: from })) library.setFilter({ kind: "tag", tag: to });
  }
}

function TagRow({ tag, editing, onEdit, onDone }: {
  tag: TagCount;
  editing: boolean;
  onEdit: (edit: TagEdit) => void;
  onDone: (name: string | null) => void;
}) {
  if (editing) return <TagField initial={tag.name} onDone={onDone} />;
  const rename = () => onEdit({ mode: "rename", tag: tag.name });
  return (
    <FilterRow
      filter={{ kind: "tag", tag: tag.name }}
      icon={<Tag size={16} />}
      title={tag.name}
      count={tag.count}
      onDoubleClick={rename}
      onContextMenu={(e) =>
        openContextMenu(
          e,
          [
            { text: "Rename Tag", run: rename },
            "separator",
            { text: "Delete Tag…", danger: true, run: () => void deleteTag(tag.name, tag.count) },
          ],
          "Tag",
        )
      }
    />
  );
}

/** Library filters and the tags; layout in docs/ux.md → Main window. */
export function Sidebar({ library }: { library: Library | undefined }) {
  const tagEdit = useLibrary((s) => s.tagEdit);
  const setTagEdit = useLibrary((s) => s.setTagEdit);
  const sidebar = library?.sidebar;

  const finishTag = (name: string | null) => {
    const edit = tagEdit;
    setTagEdit(null);
    if (!name || !edit) return;
    if (edit.mode === "new") void addTag(name);
    else void renameTag(edit.tag, name);
  };

  return (
    <>
      <div className="sidebar-head">
        <h1 className="sidebar-head__title">{library?.title ?? ""}</h1>
        <span className="sidebar-head__count">{library?.countLabel ?? ""}</span>
      </div>
      <Group title="Library" add={{ label: "New Project", run: () => void addProjects() }}>
        <FilterRow filter={{ kind: "all" }} icon={<FolderClosed size={16} />} title="All Projects" count={sidebar?.all ?? 0} />
        <FilterRow filter={{ kind: "starred" }} icon={<Star size={16} />} title="Starred" count={sidebar?.starred ?? 0} />
      </Group>
      <Group title="Tags" add={{ label: "New Tag", run: () => setTagEdit({ mode: "new" }) }}>
        {sidebar?.tags.map((tag) => (
          <TagRow
            key={tag.name}
            tag={tag}
            editing={tagEdit?.mode === "rename" && tagEdit.tag === tag.name}
            onEdit={setTagEdit}
            onDone={finishTag}
          />
        ))}
        {tagEdit?.mode === "new" && <TagField initial="" onDone={finishTag} />}
        {sidebar?.untagged != null && (
          <FilterRow filter={{ kind: "untagged" }} icon={<Tags size={16} />} title="Untagged" count={sidebar.untagged} />
        )}
      </Group>
      <Group>
        <FilterRow filter={{ kind: "archived" }} icon={<Archive size={16} />} title="Archived" count={sidebar?.archived ?? 0} />
      </Group>
      <button type="button" className="icon-btn sidebar__settings" aria-label="Settings" title="Settings" onClick={() => void api.runCommand("app:settings")}>
        <Settings size={16} />
      </button>
    </>
  );
}
