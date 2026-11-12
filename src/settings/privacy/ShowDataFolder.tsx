import { api } from "../../api";
import { attempt } from "../../state/attempt";

/** Shows Launcher's data folder in the Finder (`app:data`). */
export function ShowDataFolder() {
  return (
    <button type="button" className="btn" onClick={() => void attempt(api.runCommand("app:data"), "Could not show the data folder")}>
      Show Data Folder
    </button>
  );
}
