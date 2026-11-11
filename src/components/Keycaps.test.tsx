import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Keycaps } from "./Keycaps";

describe("Keycaps", () => {
  it("shows each symbol of the binding as its own key and names the group by the joined label", () => {
    const { container } = render(<Keycaps binding="Shift+Mod+T" />);

    expect(screen.getByLabelText("⇧⌘T")).toBeInTheDocument();
    expect([...container.querySelectorAll("kbd")].map((k) => k.textContent)).toEqual(["⇧", "⌘", "T"]);
  });

  it("renders nothing without a binding", () => {
    const { container } = render(<Keycaps binding={null} />);

    expect(container).toBeEmptyDOMElement();
  });
});
