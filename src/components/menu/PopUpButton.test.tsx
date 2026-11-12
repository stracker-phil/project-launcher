import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { resetStore } from "../../state/store";
import { MenuHost, PopUpButton } from ".";

const OPTIONS = [
  { value: "docker", label: "Docker Desktop" },
  { value: "orbstack", label: "OrbStack" },
  { value: "colima", label: "Colima" },
];

function renderButton(props: Partial<Parameters<typeof PopUpButton<string>>[0]> = {}) {
  const onChange = vi.fn();
  render(
    <>
      <label htmlFor="docker-app">Docker app</label>
      <PopUpButton id="docker-app" value="orbstack" options={OPTIONS} onChange={onChange} {...props} />
      <MenuHost />
    </>,
  );
  return onChange;
}

const button = () => screen.getByRole("button", { name: "Docker app" });

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
});

describe("PopUpButton", () => {
  it("is named by its label and closed at first", () => {
    renderButton();

    expect(button()).toHaveAttribute("aria-haspopup", "menu");
    expect(button()).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("opens a menu named after its label, with a row per choice and the current one checked", async () => {
    renderButton();

    await userEvent.click(button());

    const menu = within(screen.getByRole("menu", { name: "Docker app" }));
    expect(menu.getAllByRole("menuitemradio").map((row) => row.textContent)).toEqual(["Docker Desktop", "OrbStack", "Colima"]);
    expect(menu.getByRole("menuitemradio", { name: "OrbStack" })).toHaveAttribute("aria-checked", "true");
    expect(menu.getAllByRole("menuitemradio", { checked: true })).toHaveLength(1);
    expect(button()).toHaveAttribute("aria-expanded", "true");
  });

  it("reports the chosen value and closes the menu", async () => {
    const onChange = renderButton();
    await userEvent.click(button());

    await userEvent.click(screen.getByRole("menuitemradio", { name: "Colima" }));

    expect(onChange).toHaveBeenCalledWith("colima");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("reports nothing when the current choice is chosen again", async () => {
    const onChange = renderButton();
    await userEvent.click(button());

    await userEvent.click(screen.getByRole("menuitemradio", { name: "OrbStack" }));

    expect(onChange).not.toHaveBeenCalled();
  });

  it("closes on Escape without a change and returns the focus to the button", async () => {
    const onChange = renderButton();
    await userEvent.click(button());

    await userEvent.keyboard("{Escape}");

    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(onChange).not.toHaveBeenCalled();
    expect(button()).toHaveFocus();
  });

  it.each(["{ArrowDown}", "{ArrowUp}"])("opens on %s while it has the focus", async (key) => {
    renderButton();
    button().focus();

    await userEvent.keyboard(key);

    expect(screen.getByRole("menu", { name: "Docker app" })).toBeInTheDocument();
  });

  it("does not open while disabled", async () => {
    renderButton({ disabled: true });

    await userEvent.click(button());

    expect(button()).toBeDisabled();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });
});
