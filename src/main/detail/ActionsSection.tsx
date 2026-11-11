import type { CSSProperties } from "react";
import type { Action, ProjectConfig } from "../../api";
import { iconFor, colorVar } from "../../lib/icons";
import { bindingLabel } from "../../commands/registry";

function ActionTile({ action }: { action: Action }) {
  const Icon = iconFor(action.icon);
  return (
    <div className="action-tile" style={action.color ? ({ "--tile-color": colorVar(action.color) } as CSSProperties) : undefined}>
      <Icon size={16} aria-hidden className="action-tile__icon" />
      <span className="action-tile__label">{action.label}</span>
      {action.shortcut && <span className="action-tile__key">{bindingLabel(action.shortcut)}</span>}
    </div>
  );
}

export function ActionsSection({ config }: { config: ProjectConfig }) {
  return (
    <>
      <section className="detail-section">
        <h2 className="detail-section__title">Open</h2>
        {config.apps.length === 0 ? (
          <p className="detail-section__empty">Nothing to open yet.</p>
        ) : (
          <div className="action-grid">
            {config.apps.map((app) => (
              <ActionTile key={`${app.edit.source.list}-${app.edit.source.index}`} action={app} />
            ))}
          </div>
        )}
      </section>
      {config.scripts.length > 0 && (
        <section className="detail-section">
          <h2 className="detail-section__title">Scripts</h2>
          <div className="script-list">
            {config.scripts.map((script) => (
              <div key={`${script.edit.source.list}-${script.edit.source.index}`} className="script-row">
                <span className="script-row__label">{script.label}</span>
                <span className="script-row__command mono" title={script.command ?? ""}>{script.command}</span>
              </div>
            ))}
          </div>
        </section>
      )}
    </>
  );
}
