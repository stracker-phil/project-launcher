import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Toasts } from "../components/Toasts";
import { resetStore } from "../state/store";
import { TextField, useFieldSave } from "./fields";

const save = vi.fn<(value: string) => Promise<unknown>>();

function Field({ value = "" }: { value?: string }) {
  const [error, commit] = useFieldSave(save, "Couldn’t change the demo setting");
  return <TextField label="Demo setting" description="Used for demos." placeholder="Demo app" error={error} value={value} onCommit={commit} />;
}

const field = () => screen.getByRole("textbox", { name: "Demo setting" });

const refusal = (message: string) => ({ kind: "invalid_input", message: "Invalid value", code: null, field: { path: "demo", message } });

function renderField(value?: string) {
  render(
    <>
      <Field value={value} />
      <Toasts />
    </>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  save.mockResolvedValue(undefined);
});

describe("TextField", () => {
  it("is named by its label and shows the stored value", () => {
    renderField("Demo App");

    expect(field()).toHaveValue("Demo App");
    expect(field()).toHaveAttribute("placeholder", "Demo app");
    expect(screen.getByText("Used for demos.")).toBeInTheDocument();
  });

  it("commits the text when it is left", async () => {
    renderField();

    await userEvent.type(field(), "Demo App");
    expect(save).not.toHaveBeenCalled();
    await userEvent.tab();

    expect(save).toHaveBeenCalledWith("Demo App");
  });

  it("commits the text on Enter", async () => {
    renderField();

    await userEvent.type(field(), "Demo App{Enter}");

    expect(save).toHaveBeenCalledWith("Demo App");
  });

  it("is valid until Rust refuses a value", () => {
    renderField();

    expect(field()).not.toHaveAttribute("aria-invalid");
  });
});

describe("useFieldSave", () => {
  it("shows a refusal naming a field below the field and marks it invalid, without a toast", async () => {
    save.mockRejectedValue(refusal("No app named Demo App"));
    renderField();

    await userEvent.type(field(), "Demo App{Enter}");

    expect(await screen.findByText("No app named Demo App")).toBeInTheDocument();
    expect(field()).toHaveAttribute("aria-invalid", "true");
    expect(field()).toHaveAccessibleDescription("No app named Demo App");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("sends any other error to a toast that starts with the failure, leaving the field valid", async () => {
    save.mockRejectedValue({ kind: "io", message: "Disk full", code: null });
    renderField();

    await userEvent.type(field(), "Demo App{Enter}");

    expect(await screen.findByRole("alert")).toHaveTextContent("Couldn’t change the demo setting: Disk full");
    expect(field()).not.toHaveAttribute("aria-invalid");
  });

  it("clears the message when a later save goes through", async () => {
    save.mockRejectedValueOnce(refusal("No app named Demo App"));
    renderField();
    await userEvent.type(field(), "Demo App{Enter}");
    await screen.findByText("No app named Demo App");

    await userEvent.clear(field());
    await userEvent.type(field(), "Other App{Enter}");

    await waitFor(() => expect(screen.queryByText("No app named Demo App")).not.toBeInTheDocument());
    expect(field()).not.toHaveAttribute("aria-invalid");
  });

  it("replaces the message with the next refusal", async () => {
    save.mockRejectedValueOnce(refusal("No app named Demo App")).mockRejectedValueOnce(refusal("Not an app: Other"));
    renderField();
    await userEvent.type(field(), "Demo App{Enter}");
    await screen.findByText("No app named Demo App");

    await userEvent.clear(field());
    await userEvent.type(field(), "Other{Enter}");

    expect(await screen.findByText("Not an app: Other")).toBeInTheDocument();
    expect(screen.queryByText("No app named Demo App")).not.toBeInTheDocument();
  });

  it("keeps the message after a non-field error", async () => {
    save.mockRejectedValueOnce(refusal("No app named Demo App")).mockRejectedValueOnce({ kind: "io", message: "Disk full", code: null });
    renderField();
    await userEvent.type(field(), "Demo App{Enter}");
    await screen.findByText("No app named Demo App");

    await userEvent.clear(field());
    await userEvent.type(field(), "Other{Enter}");

    await screen.findByRole("alert");
    expect(screen.getByText("No app named Demo App")).toBeInTheDocument();
  });
});
