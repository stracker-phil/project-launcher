import { useEffect, useRef, useState } from "react";
import { Tag } from "lucide-react";
import type { Project } from "../../api";
import { openMenu } from "../../components/menu";
import { restoreFocus } from "../../lib/focus";
import { setTag } from "../actions";
import { tagMenu } from "../projectMenu";

/** The tag, or "Add Tag": a click offers the tags; "New Tag…" turns it into a field. */
export function TagPill({ project, tags, editing, onEditingChange, readOnly }: {
  project: Project;
  tags: string[];
  editing: boolean;
  onEditingChange: (editing: boolean) => void;
  readOnly: boolean;
}) {
  const [name, setName] = useState("");
  const field = useRef<HTMLInputElement>(null);
  const done = useRef(false);
  const tag = project.meta.tag;

  useEffect(() => {
    if (!editing) return;
    done.current = false;
    setName("");
    field.current?.focus();
  }, [editing]);

  if (readOnly)
    return tag ? (
      <span className="tag-pill tag-pill--filled">
        <Tag size={11} aria-hidden />
        {tag}
      </span>
    ) : null;

  if (editing) {
    const finish = (value: string | null) => {
      if (done.current) return;
      done.current = true;
      onEditingChange(false);
      if (value?.trim()) void setTag(project, value.trim());
      restoreFocus();
    };
    return (
      <input
        ref={field}
        className="tag-pill tag-pill--field"
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
    );
  }

  return (
    <button
      type="button"
      className={tag ? "tag-pill tag-pill--filled" : "tag-pill"}
      aria-haspopup="menu"
      onClick={(e) => openMenu(tagMenu(project, tags), e.currentTarget, "Tag")}
    >
      <Tag size={11} aria-hidden />
      {tag ?? "Add Tag"}
    </button>
  );
}
