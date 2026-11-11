const countFormat = new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 });
// The words follow the app's language, which is English; numbers follow the region.
const relative = new Intl.RelativeTimeFormat("en", { numeric: "auto" });

const UNITS = ["byte", "kilobyte", "megabyte", "gigabyte"] as const;
const byteFormats = UNITS.map((unit) => new Intl.NumberFormat(undefined, { style: "unit", unit, unitDisplay: "short", maximumFractionDigits: 1 }));

/** 1536 → "1.5 kB". */
export function bytes(value: number): string {
  let unit = 0;
  let amount = value;
  while (amount >= 1024 && unit < UNITS.length - 1) {
    amount /= 1024;
    unit += 1;
  }
  return byteFormats[unit].format(amount);
}

export const count = (value: number) => countFormat.format(value);

const STEPS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["year", 365 * 24 * 3600],
  ["month", 30 * 24 * 3600],
  ["week", 7 * 24 * 3600],
  ["day", 24 * 3600],
  ["hour", 3600],
  ["minute", 60],
];

/** "5 minutes ago", "last week"; "now" under a minute. */
export function ago(ms: number, now = Date.now()): string {
  const seconds = Math.round((ms - now) / 1000);
  const step = STEPS.find(([, size]) => Math.abs(seconds) >= size);
  return step ? relative.format(Math.round(seconds / step[1]), step[0]) : relative.format(0, "second");
}

const shortFormat = new Intl.NumberFormat("en", { notation: "compact", maximumFractionDigits: 1 });

/** 1834 → "1.8K", 12000 → "12K", 1200000 → "1.2M". */
export const short = (value: number) => shortFormat.format(value);

const monthYearFormat = new Intl.DateTimeFormat("en-US", { month: "short", year: "numeric" });

/** "Mar 2024". */
export const monthYear = (ms: number) => monthYearFormat.format(ms);
