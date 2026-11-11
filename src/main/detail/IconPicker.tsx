import { useRef, type CSSProperties } from "react";
import { Check } from "lucide-react";
import type { Project } from "../../api";
import { Popover } from "../../components/Popover";
import { colorVar, iconFor, PROJECT_COLORS, PROJECT_ICONS, ProjectIcon } from "../../lib/icons";
import { setMeta } from "../actions";

/** The project's tile; a click opens colours and icons, and each pick applies at once. */
export function IconPicker({ project, open, onOpenChange, readOnly }: {
  project: Project;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  readOnly: boolean;
}) {
  const tile = useRef<HTMLButtonElement>(null);
  const { icon, color } = project.meta;
  const icons = PROJECT_ICONS.includes(icon) ? PROJECT_ICONS : [icon, ...PROJECT_ICONS];
  if (readOnly) return <ProjectIcon icon={icon} color={color} size={56} />;
  return (
    <>
      <button ref={tile} type="button" className="icon-picker__tile" aria-label="Change Icon & Color" title="Change Icon & Color" onClick={() => onOpenChange(!open)}>
        <ProjectIcon icon={icon} color={color} size={56} />
      </button>
      <Popover open={open} anchor={tile} label="Icon and Color" onClose={() => onOpenChange(false)}>
        <div className="icon-picker">
          <div className="popover__caps">Color</div>
          <div className="icon-picker__colors">
            {PROJECT_COLORS.map((name) => (
              <button
                key={name}
                type="button"
                className="icon-picker__swatch"
                style={{ background: colorVar(name) }}
                aria-label={name}
                aria-pressed={name === color}
                onClick={() => void setMeta(project, { field: "color", value: name })}
              >
                {name === color && <Check size={14} aria-hidden />}
              </button>
            ))}
          </div>
          <div className="icon-picker__line" />
          <div className="popover__caps">Icon</div>
          <div className="icon-picker__icons" style={{ "--tile-color": colorVar(color) } as CSSProperties}>
            {icons.map((name) => {
              const Icon = iconFor(name);
              return (
                <button
                  key={name}
                  type="button"
                  className="icon-picker__icon"
                  aria-label={name}
                  title={name}
                  aria-pressed={name === icon}
                  onClick={() => void setMeta(project, { field: "icon", value: name })}
                >
                  <Icon size={18} aria-hidden />
                </button>
              );
            })}
          </div>
        </div>
      </Popover>
    </>
  );
}
