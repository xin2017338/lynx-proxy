import { RangeSetBuilder } from '@codemirror/state'
import { Decoration, EditorView, ViewPlugin, type ViewUpdate } from '@codemirror/view'

import { validateDsl } from './dslWasm'

/** Highlight colors for DSL tokens (each URL part uses a distinct chart/destructive token). */
export const dslHighlightColors = {
  keyword: 'var(--color-chart-2)',
  scheme: 'var(--color-chart-4)',
  host: 'var(--color-chart-1)',
  fuzzyKeyword: 'var(--color-secondary-foreground)',
  port: 'var(--color-chart-5)',
  path: 'var(--color-chart-3)',
  query: 'var(--color-chart-6)',
  paren: 'var(--color-muted-foreground)',
  comment: 'var(--color-accent-foreground)',
  cliFlag: 'var(--color-primary)',
  cliValue: 'var(--color-destructive)',
} as const

const highlightClassByKind: Record<string, string> = {
  AndOp: 'cm-dsl-keyword',
  OrOp: 'cm-dsl-keyword',
  NotOp: 'cm-dsl-keyword',
  Scheme: 'cm-dsl-scheme',
  Host: 'cm-dsl-host',
  FuzzyKeyword: 'cm-dsl-fuzzy-keyword',
  Port: 'cm-dsl-port',
  Path: 'cm-dsl-path',
  Query: 'cm-dsl-query',
  LineComment: 'cm-dsl-comment',
  ShortFlag: 'cm-dsl-cli-flag',
  LongFlag: 'cm-dsl-cli-flag',
  CliValue: 'cm-dsl-cli-value',
  Paren: 'cm-dsl-paren',
}

function buildHighlightDecorations(doc: string) {
  const builder = new RangeSetBuilder<Decoration>()
  const { highlights } = validateDsl(doc)

  for (const span of highlights) {
    const className = highlightClassByKind[span.kind]
    if (!className || span.to <= span.from) {
      continue
    }
    builder.add(
      span.from,
      span.to,
      Decoration.mark({ class: className }),
    )
  }

  return builder.finish()
}

const dslHighlightPlugin = ViewPlugin.fromClass(class {
  decorations

  constructor(view: EditorView) {
    this.decorations = buildHighlightDecorations(view.state.doc.toString())
  }

  update(update: ViewUpdate) {
    if (update.docChanged || update.viewportChanged) {
      this.decorations = buildHighlightDecorations(update.state.doc.toString())
    }
  }
}, {
  decorations: value => value.decorations,
})

const dslHighlightTheme = EditorView.baseTheme({
  '.cm-dsl-keyword': {
    color: dslHighlightColors.keyword,
    fontWeight: '700',
  },
  '.cm-dsl-scheme': {
    color: dslHighlightColors.scheme,
    fontWeight: '700',
  },
  '.cm-dsl-host': {
    color: dslHighlightColors.host,
    fontWeight: '650',
  },
  '.cm-dsl-fuzzy-keyword': {
    color: dslHighlightColors.fuzzyKeyword,
    fontWeight: '650',
    textDecoration: 'underline',
    textDecorationThickness: '1px',
  },
  '.cm-dsl-port': {
    color: dslHighlightColors.port,
    fontWeight: '600',
  },
  '.cm-dsl-path': {
    color: dslHighlightColors.path,
  },
  '.cm-dsl-query': {
    color: dslHighlightColors.query,
  },
  '.cm-dsl-comment': {
    color: dslHighlightColors.comment,
    fontStyle: 'italic',
  },
  '.cm-dsl-cli-flag': {
    color: dslHighlightColors.cliFlag,
    fontWeight: '600',
  },
  '.cm-dsl-cli-value': {
    color: dslHighlightColors.cliValue,
  },
  '.cm-dsl-paren': {
    color: dslHighlightColors.paren,
  },
})

export const dslLanguageExtension = [
  dslHighlightPlugin,
  dslHighlightTheme,
]
