// The Markdown twin of a report: the same fields as the HTML slides, top to
// bottom, as a document plannotator-tui can open in herdr, where the comments
// made on it go back to the task's Claude. Pure, like report.ts, so the unit
// tests reach it directly.

import type { Plan } from './plan'
import type { CodePair, Diagram, Report, Section, Tool } from './report'

// The twin's path: the report's, with .md for .html.
export const markdownPath = (htmlPath: string): string => `${htmlPath.replace(/\.html$/, '')}.md`

// A fence longer than any run of backticks in the text, so code that holds
// ``` stays one block.
const fence = (text: string, lang = ''): string => {
  const longest = Math.max(2, ...(text.match(/`+/g) ?? []).map(run => run.length))
  const bar = '`'.repeat(longest + 1)
  return `${bar}${lang}\n${text}\n${bar}`
}

const cell = (text: string): string => text.replace(/\|/g, '\\|')

const table = (head: readonly string[], rows: readonly (readonly string[])[]): string =>
  [
    `| ${head.join(' | ')} |`,
    `|${head.map(() => ' --- ').join('|')}|`,
    ...rows.map(row => `| ${row.map(cell).join(' | ')} |`),
  ].join('\n')

const bullets = (lines: readonly string[]): string => lines.map(line => `- ${line}`).join('\n')

const drawing = (diagram: Diagram): string => fence(diagram.mermaid, 'mermaid')

const code = (pairs: readonly CodePair[]): string =>
  pairs
    .map(pair =>
      [
        `### ${pair.file}`,
        pair.before === '' ? '_New file._' : `Before:\n\n${fence(pair.before)}`,
        pair.after === '' ? '_Removed._' : `After:\n\n${fence(pair.after)}`,
      ].join('\n\n'),
    )
    .join('\n\n')

const tools = (list: readonly Tool[]): string =>
  table(['Tool', 'How it connects', 'Swap', 'Why'], list.map(t => [t.tool, t.how, t.swap, t.why]))

const part = (s: Section): string => {
  const out = [`## ${s.heading}`]
  if (s.look_at !== undefined) out.push(`_${s.look_at}_`)
  if (s.diagram !== undefined) out.push(drawing(s.diagram))
  if (s.before !== undefined && s.after !== undefined) {
    out.push(`Before:\n\n${drawing(s.before)}`, `After:\n\n${drawing(s.after)}`)
  }
  out.push(bullets(s.points))
  if (s.code !== undefined) out.push(code(s.code))
  if (s.tools !== undefined) out.push(tools(s.tools))
  if (s.detail !== undefined) out.push(`More detail:\n\n${s.detail}`)
  return out.join('\n\n')
}

// The report as Markdown, in the slides' order, ending with the plan the
// person was shown, as the HTML's last slide does.
export const reportMarkdown = (report: Report, plan: Plan): string => {
  const out = [`# ${report.title}`, report.summary, `## What changes\n\n${bullets(report.changes)}`]
  if (report.unchanged.length > 0) out.push(`## What stays the same\n\n${bullets(report.unchanged)}`)
  if (report.needs_your_eye.length > 0) {
    out.push(`## Where your judgement is needed\n\n${bullets(report.needs_your_eye)}`)
  }
  if (report.terms.length > 0) {
    out.push(`## Words used here\n\n${bullets(report.terms.map(t => `**${t.term}**: ${t.meaning}`))}`)
  }
  out.push(...report.sections.map(part))
  if (report.files.length > 0) {
    out.push(`## Files\n\n${table(['Change', 'File', 'Why'], report.files.map(f => [f.change, `\`${f.path}\``, f.why]))}`)
  }
  out.push(`## Done when\n\n${table(['Check', 'How it is checked'], report.checks.map(c => [c.check, c.how]))}`)
  const notes = plan.notes.length === 0 ? 'No notes.' : plan.notes.map((n, i) => `${i + 1}. ${n}`).join('\n')
  out.push(`## The plan\n\n**Description:** ${plan.description}\n\n${notes}`)
  return `${out.join('\n\n')}\n`
}
