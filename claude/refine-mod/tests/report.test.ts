import { describe, expect, test } from 'claude-code/testing'
import {
  CSP,
  MAX_BODY,
  MAX_TITLE,
  MERMAID_SRI,
  MERMAID_URL,
  openArgv,
  parseReport,
  reportPage,
  reportPath,
  reportsDir,
} from '../hooks/report'

const UUID = '0b8f6a52-3c4d-4e5f-8a9b-0c1d2e3f4a5b'

describe('parseReport', () => {
  test('takes a title and a body', () => {
    expect(parseReport({ title: 'feat: A plan', body: '<h1>A plan</h1>' })).toEqual({
      title: 'feat: A plan',
      body: '<h1>A plan</h1>',
    })
  })

  test('refuses a missing, empty, multi-line, hidden or long title', () => {
    expect(parseReport({ body: '<p>x</p>' })).toBe('title must be a non-empty string')
    expect(parseReport({ title: ' ', body: '<p>x</p>' })).toBe('title must be a non-empty string')
    expect(parseReport({ title: 'a\nb', body: '<p>x</p>' })).toBe('title must be one line')
    expect(parseReport({ title: 'a‮b', body: '<p>x</p>' })).toBe(
      'title must be plain text: no control or invisible characters',
    )
    expect(parseReport({ title: 'x'.repeat(MAX_TITLE + 1), body: '<p>x</p>' })).toBe(
      `title must be at most ${MAX_TITLE} characters`,
    )
  })

  test('refuses a missing, empty or overlong body', () => {
    expect(parseReport({ title: 't' })).toBe('body must be a non-empty string')
    expect(parseReport({ title: 't', body: '  ' })).toBe('body must be a non-empty string')
    expect(parseReport({ title: 't', body: 'x'.repeat(MAX_BODY + 1) })).toBe(
      `body must be at most ${MAX_BODY} characters`,
    )
  })

  test('refuses every element the page must not hold, by name, any case or spacing', () => {
    const tags = ['script', 'meta', 'base', 'link', 'iframe', 'frame', 'frameset', 'object', 'embed', 'form', 'portal']
    for (const tag of tags) {
      expect(parseReport({ title: 't', body: `<p>x</p><${tag} a="b">` })).toBe(
        `body must not contain <${tag}>: the page runs no script but Mermaid, loads nothing and sends nothing`,
      )
    }
    expect(parseReport({ title: 't', body: '<svg>< SCRIPT>x</script></svg>' })).toContain('<script>')
    expect(parseReport({ title: 't', body: '<META http-equiv="refresh">' })).toContain('<meta>')
  })

  test('keeps words that only begin like a refused element', () => {
    for (const body of ['<p>a <code>&lt;script&gt;</code></p>', '<formula>x</formula>', '<p>metadata</p>', '<baseline/>']) {
      expect(parseReport({ title: 't', body })).toEqual({ title: 't', body })
    }
  })
})

describe('reportPage', () => {
  const page = reportPage({ title: 'feat: <A> & "B"', body: '<pre class="mermaid">flowchart LR\n a --> b</pre>' })

  test('is a whole document with the body verbatim', () => {
    expect(page.startsWith('<!doctype html>\n')).toBe(true)
    expect(page).toContain('<pre class="mermaid">flowchart LR\n a --> b</pre>')
    expect(page.trimEnd().endsWith('</html>')).toBe(true)
  })

  test('puts the policy first in the head, before anything it governs', () => {
    const csp = page.indexOf(`<meta http-equiv="Content-Security-Policy" content="${CSP}">`)
    expect(csp).toBeGreaterThan(-1)
    expect(csp).toBeLessThan(page.indexOf('<title>'))
    expect(csp).toBeLessThan(page.indexOf('<style>'))
    expect(csp).toBeLessThan(page.indexOf('<script'))
    expect(csp).toBeLessThan(page.indexOf('<body>'))
  })

  test('allows no script but pinned Mermaid, and nothing fetched or sent', () => {
    expect(CSP).toBe(
      `default-src 'none'; script-src ${MERMAID_URL}; style-src 'unsafe-inline'; ` +
        "img-src data:; font-src data:; form-action 'none'; base-uri 'none'",
    )
    expect(MERMAID_URL).toBe('https://cdn.jsdelivr.net/npm/mermaid@11.17.2/dist/mermaid.min.js')
    expect(page).toContain(
      `<script src="${MERMAID_URL}" integrity="${MERMAID_SRI}" crossorigin="anonymous"></script>`,
    )
  })

  test('escapes the title', () => {
    expect(page).toContain('<title>feat: &lt;A&gt; &amp; &quot;B&quot;</title>')
  })
})

describe('reportsDir', () => {
  test('takes an absolute folder, without a trailing slash', () => {
    expect(reportsDir('/home/x/.local/share/niri-tasks/reviews')).toBe('/home/x/.local/share/niri-tasks/reviews')
    expect(reportsDir('/r/')).toBe('/r')
  })
  test('refuses the rest', () => {
    for (const value of [undefined, '', 'reviews', '~/reviews', '/', 7]) expect(reportsDir(value)).toBeUndefined()
  })
})

test('reportPath names the task and the UTC time', () => {
  expect(reportPath('/r', UUID, Date.UTC(2026, 9, 6, 1, 2, 3, 456))).toBe('/r/refine-0b8f6a52-20261006-010203.html')
  expect(reportPath('/r', UUID.toUpperCase(), 0)).toBe('/r/refine-0b8f6a52-19700101-000000.html')
})

test('openArgv passes the path as an argument, never as script', () => {
  const path = '/r/it\'s "here" $(x).html'
  const argv = openArgv(path)
  expect(argv).toEqual(['sh', '-c', 'xdg-open "$1" >/dev/null 2>&1 </dev/null &', 'sh', path])
})
