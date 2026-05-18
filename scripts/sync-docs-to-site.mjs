#!/usr/bin/env node
/**
 * Copies root docs/*.md into docs-site/guide/ for VitePress.
 */
import { copyFile, mkdir, readdir } from 'node:fs/promises'
import { join } from 'node:path'

const root = join(import.meta.dirname, '..')
const srcDir = join(root, 'docs')
const destDir = join(root, 'docs-site', 'guide')

const files = await readdir(srcDir)
await mkdir(destDir, { recursive: true })

for (const name of files) {
  if (!name.endsWith('.md')) {
    continue
  }
  const base = name.replace(/\.md$/, '')
  await copyFile(join(srcDir, name), join(destDir, `${base}.md`))
}

console.log(`Synced ${files.filter((f) => f.endsWith('.md')).length} guides → docs-site/guide/`)
