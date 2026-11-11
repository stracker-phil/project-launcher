import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatesRow } from "./StatesRow";

describe("StatesRow", () => {
  it("shows a chip with label and value per state", () => {
    render(<StatesRow states={[{ label: "Branch", value: "main" }, { label: "Server", value: "Running", color: "Green" }]} />);

    const chips = within(screen.getByRole("list", { name: "States" })).getAllByRole("listitem");
    expect(chips.map((chip) => chip.textContent)).toEqual(["Branchmain", "ServerRunning"]);
    expect(within(chips[0]).getByText("Branch")).toBeInTheDocument();
    expect(within(chips[0]).getByText("main")).toBeInTheDocument();
  });

  it("shows nothing without states", () => {
    render(<StatesRow states={[]} />);

    expect(screen.queryByRole("list")).not.toBeInTheDocument();
  });
});
