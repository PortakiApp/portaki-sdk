import { copyFileSync, mkdirSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const source = join(root, '../../schema/module.v1.json')
const targetDir = join(root, 'schema')
const target = join(targetDir, 'module.v1.json')

mkdirSync(targetDir, { recursive: true })
copyFileSync(source, target)
