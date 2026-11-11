import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../api", async (orig) => ({ ...(await orig<object>()), api: (await import("../../test/apiMock")).createApiMock() }));

import { api, type Project } from "../../api";
import { MenuHost } from "../../components/menu";
import { resetLibrary, useLibrary } from "../../state/library";
import { resetStore } from "../../state/store";
import { project } from "../../test/library";
import { TagPill } from "./TagPill";

const mocked = vi.mocked(api, true);
const TAGS = ["Client", "Internal"];

function renderPill(subject: Project, props: { editing?: boolean; readOnly?: boolean } = {}) {
  const onEditingChange = vi.fn();
  render(
    <>
      <TagPill project={subject} tags={TAGS} editing={props.editing ?? false} onEditingChange={onEditingChange} readOnly={props.readOnly ?? false} />
      <MenuHost />
    </>,
  );
  return { onEditingChange };
}

beforeEach(() => {
  vi.clearAllMocks();
  resetStore();
  resetLibrary();
});

describe("TagPill", () => {
  describe("showing", () => {
    it("offers Add Tag while the project has no tag", () => {
      renderPill(project());

      expect(screen.getByRole("button", { name: "Add Tag" })).toBeInTheDocument();
    });

    it("shows the project's tag", () => {
      renderPill(project({ meta: { tag: "Client" } }));

      expect(screen.getByRole("button", { name: "Client" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Add Tag" })).not.toBeInTheDocument();
    });
  });

  describe("menu", () => {
    it("lists No Tag, each tag with the current one checked, and New Tag…", async () => {
      renderPill(project({ meta: { tag: "Client" } }));

      await userEvent.click(screen.getByRole("button", { name: "Client" }));

      const menu = screen.getByRole("menu", { name: "Tag" });
      expect(within(menu).getByRole("menuitemradio", { name: "No Tag" })).toHaveAttribute("aria-checked", "false");
      expect(within(menu).getByRole("menuitemradio", { name: "Client" })).toHaveAttribute("aria-checked", "true");
      expect(within(menu).getByRole("menuitemradio", { name: "Internal" })).toHaveAttribute("aria-checked", "false");
      expect(within(menu).getByRole("menuitem", { name: "New Tag…" })).toBeInTheDocument();
    });

    it("checks No Tag for an untagged project", async () => {
      renderPill(project());

      await userEvent.click(screen.getByRole("button", { name: "Add Tag" }));

      expect(screen.getByRole("menuitemradio", { name: "No Tag" })).toHaveAttribute("aria-checked", "true");
    });

    it("sets the chosen tag", async () => {
      const subject = project();
      renderPill(subject);

      await userEvent.click(screen.getByRole("button", { name: "Add Tag" }));
      await userEvent.click(screen.getByRole("menuitemradio", { name: "Internal" }));

      expect(mocked.setProjectMeta).toHaveBeenCalledWith(subject.id, { field: "tag", value: "Internal" });
      expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    });

    it("clears the tag with No Tag", async () => {
      const subject = project({ meta: { tag: "Client" } });
      renderPill(subject);

      await userEvent.click(screen.getByRole("button", { name: "Client" }));
      await userEvent.click(screen.getByRole("menuitemradio", { name: "No Tag" }));

      expect(mocked.setProjectMeta).toHaveBeenCalledWith(subject.id, { field: "tag", value: null });
    });

    it("asks to edit the tag field with New Tag…", async () => {
      const subject = project();
      renderPill(subject);

      await userEvent.click(screen.getByRole("button", { name: "Add Tag" }));
      await userEvent.click(screen.getByRole("menuitem", { name: "New Tag…" }));

      expect(useLibrary.getState().editing).toEqual({ id: subject.id, field: "tag" });
      expect(mocked.setProjectMeta).not.toHaveBeenCalled();
    });
  });

  describe("editing", () => {
    it("shows an empty Tag name field instead of the button", () => {
      renderPill(project(), { editing: true });

      expect(screen.getByRole("textbox", { name: "Tag name" })).toHaveValue("");
      expect(screen.queryByRole("button", { name: "Add Tag" })).not.toBeInTheDocument();
    });

    it("sets the trimmed new tag on Enter and ends editing", async () => {
      const subject = project();
      const { onEditingChange } = renderPill(subject, { editing: true });

      await userEvent.type(screen.getByRole("textbox", { name: "Tag name" }), "  Docs  {Enter}");

      expect(mocked.setProjectMeta).toHaveBeenCalledWith(subject.id, { field: "tag", value: "Docs" });
      expect(onEditingChange).toHaveBeenCalledWith(false);
    });

    it("sets the new tag when the field loses focus", async () => {
      const subject = project();
      renderPill(subject, { editing: true });

      await userEvent.type(screen.getByRole("textbox", { name: "Tag name" }), "Docs");
      await userEvent.tab();

      expect(mocked.setProjectMeta).toHaveBeenCalledWith(subject.id, { field: "tag", value: "Docs" });
    });

    it("sets nothing on Escape and ends editing", async () => {
      const { onEditingChange } = renderPill(project(), { editing: true });

      await userEvent.type(screen.getByRole("textbox", { name: "Tag name" }), "Docs{Escape}");

      expect(mocked.setProjectMeta).not.toHaveBeenCalled();
      expect(onEditingChange).toHaveBeenCalledWith(false);
    });

    it("sets nothing for a blank name", async () => {
      const { onEditingChange } = renderPill(project(), { editing: true });

      await userEvent.type(screen.getByRole("textbox", { name: "Tag name" }), "   {Enter}");

      expect(mocked.setProjectMeta).not.toHaveBeenCalled();
      expect(onEditingChange).toHaveBeenCalledWith(false);
    });
  });

  describe("read-only", () => {
    it("shows the tag as plain text", () => {
      renderPill(project({ locked: true, meta: { tag: "Client" } }), { readOnly: true });

      expect(screen.getByText("Client")).toBeInTheDocument();
      expect(screen.queryByRole("button")).not.toBeInTheDocument();
    });

    it("shows nothing without a tag", () => {
      renderPill(project({ locked: true }), { readOnly: true });

      expect(screen.queryByRole("button")).not.toBeInTheDocument();
      expect(screen.queryByText("Add Tag")).not.toBeInTheDocument();
    });
  });
});
