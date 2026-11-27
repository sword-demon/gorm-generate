// The in-app "Report an issue" entry opens GitHub's new-issue form with the
// facts the app already knows prefilled: version, dialect and the cache flag,
// so triage starts from a complete report. The DDL is deliberately not
// prefilled — whatever the reporter pastes becomes public, and that choice is
// theirs to make (see the warning in .github/ISSUE_TEMPLATE/bug_report.yml).

import { dialectLabel, type Dialect } from "@/lib/settings"

const NEW_ISSUE_URL = "https://github.com/sword-demon/gorm-generate/issues/new"

export interface ReportContext {
  version: string
  dialect: Dialect
  cacheEnabled: boolean
}

/** Prefilled bug-form URL; the field names match bug_report.yml's ids. */
export function buildBugReportUrl(context: ReportContext): string {
  const params = new URLSearchParams({
    template: "bug_report.yml",
    title: "[bug] ",
    version: context.version,
    dialect: dialectLabel(context.dialect),
    cache: context.cacheEnabled ? "开启（Redis）" : "关闭",
  })
  return `${NEW_ISSUE_URL}?${params}`
}
