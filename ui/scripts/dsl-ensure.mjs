import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const uiRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const repoRoot = path.resolve(uiRoot, '..')
const crateDir = path.join(repoRoot, 'crates/lynx-dsl')
const wasmDir = path.join(uiRoot, 'src/wasm/lynx-dsl')

const sourceInputs = [
  path.join(crateDir, 'Cargo.toml'),
  path.join(crateDir, 'dsl.pest'),
  path.join(crateDir, 'src'),
]

const wasmArtifacts = ['lynx_dsl_bg.wasm', 'lynx_dsl.js', 'lynx_dsl.d.ts']

function newestMtimeMs(targetPath) {
  const stat = fs.statSync(targetPath)
  if (stat.isFile()) {
    return stat.mtimeMs
  }

  if (!stat.isDirectory()) {
    return 0
  }

  let newest = stat.mtimeMs
  for (const entry of fs.readdirSync(targetPath, { withFileTypes: true })) {
    const childPath = path.join(targetPath, entry.name)
    const childNewest = newestMtimeMs(childPath)
    if (childNewest > newest) {
      newest = childNewest
    }
  }
  return newest
}

function ensureWasmFresh() {
  const missing = wasmArtifacts.filter(name => !fs.existsSync(path.join(wasmDir, name)))
  const latestSourceChange = Math.max(...sourceInputs.map(newestMtimeMs))
  const earliestArtifact = Math.min(
    ...wasmArtifacts.map(name => {
      const artifactPath = path.join(wasmDir, name)
      return fs.existsSync(artifactPath) ? fs.statSync(artifactPath).mtimeMs : 0
    }),
  )

  const stale = missing.length > 0 || earliestArtifact < latestSourceChange

  if (!stale) {
    console.log('[dsl:ensure] DSL WASM is up to date.')
    return
  }

  if (missing.length > 0) {
    console.log(`[dsl:ensure] Missing artifacts: ${missing.join(', ')}`)
  }
  else {
    console.log('[dsl:ensure] DSL sources changed, rebuilding WASM artifacts...')
  }

  const result = spawnSync('npm', ['run', 'dsl:build'], {
    cwd: uiRoot,
    stdio: 'inherit',
  })

  if (result.error) {
    console.error(result.error.message)
    process.exit(1)
  }
  if (result.status !== 0) {
    process.exit(result.status ?? 1)
  }
}

ensureWasmFresh()
