# 0046. The Settings search finds every row

## Status

Accepted (2026-11-12). Owner's decision. Extends [0033](0033-settings-is-its-own-window.md) (the search field).

## Context

The Settings search matched page titles, keywords and a hand-kept list of row titles in `src/settings/pages.ts`.
The list missed rows and could drift from the labels, and the page list showed one hit per page, so the search
read as a filter of the page list. macOS System Settings lists every matching setting and highlights the one
picked. The sibling apps match only page titles and group headings.

## Decision

- Each page module exports its searchable rows (`SettingsRow`: `id`, `title`, optional `keywords`, `under`) next
  to its component and registers them with the page. Every `Row` has an `anchor`; a row's entry carries that
  anchor as `id` and its label as `title`. Groups with an anchor are entries too. A row whose control shows only
  for some settings names the row to reveal meanwhile in `under`. Row-specific words go in the row's keywords,
  not the page's.
- A test renders every registered page and fails when a shown row and its entry disagree: an anchor without an
  entry, an entry without its row (unless `under`), or a title other than the label.
- While the field has text, the page list becomes one list of every matching page and row, ranked as in the
  palette (`src/lib/rank.ts`: title starts with the query, then a word of it, then every word in the title or
  keywords, then the title's letters in order); a row hit shows its page as the list does, the row's title on a
  second line. ↵ opens the first hit.
- Opening a row (a hit, `open_settings` with a row id) shows its page scrolled to the row and washes the row in
  the accent once (`--motion-highlight`). Each request is a new object, so picking the same hit again scrolls and
  highlights again.
- Rejected: reading the rows from the rendered pages at runtime (pages load data and show some rows only in some
  states); searching descriptions (long text matches too much; keywords hold the synonyms, as in macOS).

## Consequences

- A new row needs an anchor and an entry; the test names what is missing.
- The ranking moved from `src/palette/` to `src/lib/rank.ts`, shared by the palette and Settings.
