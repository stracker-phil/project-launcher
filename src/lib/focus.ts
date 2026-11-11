/** Where the keys go when nothing holds them: the project list, so ↑↓ keep working. */
export function focusList(): void {
  document.querySelector<HTMLElement>('[role="listbox"][aria-label="Projects"]')?.focus();
}

/** After a field or panel closes: the list takes the keys unless something else already has them. */
export function restoreFocus(): void {
  requestAnimationFrame(() => {
    const active = document.activeElement;
    if (!active || active === document.body) focusList();
  });
}
