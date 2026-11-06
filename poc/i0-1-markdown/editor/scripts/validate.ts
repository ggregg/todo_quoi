/**
 * Validation du POC I0-1, côté TipTap (Node, sans navigateur).
 *
 * Pour chaque fichier du corpus :
 *   1. Rust : Markdown → JSON                         (`mdconv to-json`)
 *   2. TipTap : le JSON est-il valide pour le schéma ? (`Node.fromJSON` + `check()`)
 *   3. TipTap : normalisation (`toJSON`, comme `editor.getJSON()`)
 *   4. Rust : JSON normalisé → Markdown               (`mdconv to-md`)
 *   5. Le Markdown obtenu est-il identique à l'original ?
 *
 * Puis un scénario d'édition : cocher une case et changer le choix radio via des
 * transactions ProseMirror (comme le ferait l'éditeur), et vérifier le Markdown produit.
 *
 * Lancement : `pnpm validate` (le binaire Rust doit être compilé : `cargo build`).
 */
import { execFileSync } from 'node:child_process'
import { readdirSync, readFileSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { getSchema } from '@tiptap/core'
import { Node as PmNode } from '@tiptap/pm/model'
import { EditorState } from '@tiptap/pm/state'
import { extensions } from '../src/extensions.ts'

const here = dirname(fileURLToPath(import.meta.url))
const crateDir = join(here, '..', '..')
const corpusDir = join(crateDir, 'tests', 'corpus')
const mdconv = join(crateDir, 'target', 'debug', process.platform === 'win32' ? 'mdconv.exe' : 'mdconv')

const schema = getSchema(extensions)

type TaskFile = { header: string | null; doc: Record<string, unknown> }

const toJson = (markdown: string): TaskFile =>
  JSON.parse(execFileSync(mdconv, ['to-json'], { input: markdown, encoding: 'utf8' }))
const toMarkdown = (file: TaskFile): string =>
  execFileSync(mdconv, ['to-md'], { input: JSON.stringify(file), encoding: 'utf8' })

/** Le JSON passe dans ProseMirror : validation stricte du schéma puis normalisation. */
function throughTipTap(file: TaskFile): { file: TaskFile; node: PmNode } {
  const node = PmNode.fromJSON(schema, file.doc)
  node.check() // lève une erreur si un contenu viole le schéma
  return { file: { ...file, doc: node.toJSON() }, node }
}

let failures = 0
const report = (ok: boolean, label: string, detail = '') => {
  if (!ok) failures++
  console.log(`${ok ? '✔' : '✘'} ${label}${detail ? `\n${detail}` : ''}`)
}

// --- 1. Aller-retour du corpus ---------------------------------------------------------
for (const name of readdirSync(corpusDir).filter((f) => f.endsWith('.md')).sort()) {
  const original = readFileSync(join(corpusDir, name), 'utf8')
  try {
    const { file } = throughTipTap(toJson(original))
    const back = toMarkdown(file)
    report(back === original, `aller-retour via TipTap : ${name}`, back === original ? '' : diff(original, back))
  } catch (error) {
    report(false, `schéma TipTap : ${name}`, String(error))
  }
}

// --- 2. Scénario d'édition -------------------------------------------------------------
{
  const original = readFileSync(join(corpusDir, '02-blocs.md'), 'utf8')
  const { file, node } = throughTipTap(toJson(original))
  let state = EditorState.create({ schema, doc: node })
  const tr = state.tr
  state.doc.descendants((child, pos) => {
    if (child.type.name === 'taskItem' && child.textContent.startsWith('Sauvegarde')) {
      tr.setNodeMarkup(pos, undefined, { ...child.attrs, checked: true })
    }
    if (child.type.name === 'radioOption') {
      tr.setNodeMarkup(pos, undefined, { ...child.attrs, checked: child.textContent === 'Production' })
    }
  })
  state = state.apply(tr)
  const edited = toMarkdown({ ...file, doc: state.doc.toJSON() })
  const expected = original
    .replace('- [ ] Sauvegarde de la base', '- [x] Sauvegarde de la base')
    .replace('- (x) Pré-production', '- ( ) Pré-production')
    .replace('- ( ) Production', '- (x) Production')
  report(edited === expected, "édition (case cochée + choix radio) reflétée dans le Markdown", edited === expected ? '' : diff(expected, edited))
}

console.log(failures === 0 ? '\nPOC I0-1 côté TipTap : OK' : `\n${failures} échec(s)`)
process.exit(failures === 0 ? 0 : 1)

/** Affiche les premières lignes qui diffèrent. */
function diff(expected: string, actual: string): string {
  const a = expected.split('\n')
  const b = actual.split('\n')
  const lines: string[] = []
  for (let i = 0; i < Math.max(a.length, b.length) && lines.length < 12; i++) {
    if (a[i] !== b[i]) lines.push(`  ligne ${i + 1}\n    attendu : ${JSON.stringify(a[i])}\n    obtenu  : ${JSON.stringify(b[i])}`)
  }
  return lines.join('\n')
}
