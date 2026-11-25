// Settings persistence: the shape written to localStorage, and the tolerant
// loader that guards against stale or corrupt values. Saving lives in
// `useSettings`, which owns the reactive refs.

export const STORAGE_KEY = "gorm-generate:state"

export const defaultDdl =
  "CREATE TABLE `users` (\n" +
  "  `id` BIGINT NOT NULL AUTO_INCREMENT,\n" +
  "  `username` VARCHAR(64) NOT NULL,\n" +
  "  `email` VARCHAR(128) NOT NULL,\n" +
  "  `age` TINYINT(1) NULL,\n" +
  "  `balance` DECIMAL(12,2) NOT NULL,\n" +
  "  `created_at` DATETIME NOT NULL,\n" +
  "  `deleted_at` DATETIME NULL,\n" +
  "  PRIMARY KEY (`id`)\n" +
  ") ENGINE=InnoDB COMMENT='用户表';"

/** The panes never shrink below this share of the container. */
export const MIN_PANE_PERCENT = 15
export const MAX_PANE_PERCENT = 85

export function clampPanePercent(value: number): number {
  if (!Number.isFinite(value)) return MIN_PANE_PERCENT
  return Math.min(MAX_PANE_PERCENT, Math.max(MIN_PANE_PERCENT, value))
}

export interface PersistedState {
  ddl: string
  dialect: "mysql" | "postgres"
  modulePath: string
  cacheEnabled: boolean
  cacheTtlSeconds: number
  tinyintAsBool: boolean
  /** Width of the SQL pane, as a share of the window. */
  panePercent: number
}

export function defaults(): PersistedState {
  return {
    ddl: defaultDdl,
    dialect: "mysql",
    modulePath: "github.com/example/app",
    cacheEnabled: false,
    cacheTtlSeconds: 300,
    tinyintAsBool: false,
    panePercent: 50,
  }
}

/** Reads the stored settings; each missing or invalid field falls back alone. */
export function loadState(raw: string | null): PersistedState {
  const base = defaults()
  if (!raw) return base
  try {
    const s = JSON.parse(raw)
    return {
      ddl: typeof s.ddl === "string" ? s.ddl : base.ddl,
      dialect: s.dialect === "postgres" ? "postgres" : "mysql",
      modulePath: typeof s.modulePath === "string" ? s.modulePath : base.modulePath,
      cacheEnabled: typeof s.cacheEnabled === "boolean" ? s.cacheEnabled : base.cacheEnabled,
      cacheTtlSeconds:
        Number.isInteger(s.cacheTtlSeconds) && s.cacheTtlSeconds > 0
          ? s.cacheTtlSeconds
          : base.cacheTtlSeconds,
      tinyintAsBool: typeof s.tinyintAsBool === "boolean" ? s.tinyintAsBool : base.tinyintAsBool,
      panePercent:
        typeof s.panePercent === "number"
          ? clampPanePercent(s.panePercent)
          : base.panePercent,
    }
  } catch {
    // Corrupt state is not worth failing the app over; use the defaults.
    return base
  }
}
