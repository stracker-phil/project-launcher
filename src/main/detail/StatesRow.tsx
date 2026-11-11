import type { StateDisplay } from "../../api";
import { colorVar } from "../../lib/icons";

/** The project's states from its state providers, as labelled chips. */
export function StatesRow({ states }: { states: StateDisplay[] }) {
  if (states.length === 0) return null;
  return (
    <div className="states-row" role="list" aria-label="States">
      {states.map((state) => (
        <span key={state.label} role="listitem" className="state-chip">
          {state.color && <span className="state-chip__dot" style={{ background: colorVar(state.color) }} aria-hidden />}
          <span className="state-chip__label">{state.label}</span>
          <span className="state-chip__value">{state.value}</span>
        </span>
      ))}
    </div>
  );
}
