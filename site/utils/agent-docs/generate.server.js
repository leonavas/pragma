import { readFile, writeFile, mkdir, readdir } from 'node:fs/promises'

const siteOrigin = 'https://pragma.leonavas.com'
const siteRoot = new URL('../../', import.meta.url)
const coreRoot = new URL('../../../core/', import.meta.url)
const templates = new URL('./', import.meta.url)
const publicRoot = new URL('public/', siteRoot)

const topics = [
  { id: 'rules', file: 'starter/AGENTS.md', title: 'App rules (AGENTS.md)', summary: 'the rules every pragma app follows; read first' },
  { id: 'structure', summary: 'where each file goes, pragma.json' },
  { id: 'syntax', summary: 'HTML directives ({path}, :, ., @, $) and blocks' },
  { id: 'components', summary: 'components, props, slots' },
  { id: 'state', summary: 'script: state, actions, derived values, mount, stores' },
  { id: 'routes', summary: 'pages, links, params, query, layouts (including nested), head tags' },
  { id: 'data', summary: 'load, server actions, skeleton, SSR' },
  { id: 'overlays', summary: 'modal, alert, dropdown' },
  { id: 'css', summary: 'styles and theme' },
  { id: 'classes', summary: 'every utility class' },
  { id: 'webmcp', summary: 'forms as WebMCP tools, markdown for agents' },
  { id: 'packages', summary: 'npm packages: when and which' },
  { id: 'performance', summary: 'performance defaults and budgets' },
  { id: 'logs', summary: 'OpenTelemetry logs' },
  { id: 'cli', summary: 'commands, self-update, upgrade, check rules, deploy' },
]

const drift = []

async function readText(url) {
  return readFile(url, 'utf8')
}

async function loadTopics() {
  const files = (await readdir(new URL('docs/', coreRoot))).filter(name => name.endsWith('.md'))
  for (const name of files) {
    if (!topics.some(topic => topic.id === name.slice(0, -3))) drift.push('core/docs/' + name + ' is not in the topics of generate.server.js')
  }
  return Promise.all(topics.map(async topic => {
    const markdown = (await readText(new URL(topic.file ?? 'docs/' + topic.id + '.md', coreRoot))).trim()
    const heading = markdown.match(/^# (.+)$/m)
    return { ...topic, markdown, title: topic.title ?? heading[1].replaceAll('`', '') }
  }))
}

function docLinks(loaded) {
  return loaded.map(topic => '- [' + topic.title + '](' + siteOrigin + '/docs/' + topic.id + '.md): ' + topic.summary).join('\n')
}

function allDocs(loaded) {
  return loaded.map(topic => 'Source: ' + siteOrigin + '/docs/' + topic.id + '.md\n\n' + topic.markdown).join('\n\n---\n\n')
}

function attributeOf(tag, name) {
  return tag.match(new RegExp('\\s' + name + '="([^"]*)"'))?.[1]
}

async function toolList(paths) {
  const sections = []
  for (const path of paths) {
    const source = await readText(new URL(path, siteRoot))
    for (const form of source.matchAll(/<form\b[^>]*>[\s\S]*?<\/form>/g)) {
      const opening = form[0].match(/<form\b[^>]*>/)[0]
      const toolName = attributeOf(opening, 'toolname')
      if (!toolName) continue
      const fields = [...form[0].matchAll(/<(?:input|select|textarea)\b[^>]*>/g)]
        .map(field => field[0])
        .filter(field => attributeOf(field, 'name'))
        .map(field => '  - `' + attributeOf(field, 'name') + '`: ' + (attributeOf(field, 'toolparamdescription') ?? ''))
      sections.push(['- `' + toolName + '`: ' + attributeOf(opening, 'tooldescription'), ...fields].join('\n'))
    }
  }
  return sections.join('\n')
}

async function sourceBlock(path) {
  return '```html\n' + (await readText(new URL(path, siteRoot))).trim() + '\n```'
}

async function fill(template, loaded) {
  let filled = template.replaceAll('{{origin}}', siteOrigin)
  filled = filled.replace('{{docs}}', () => docLinks(loaded))
  filled = filled.replace('{{all-docs}}', () => allDocs(loaded))
  for (const placeholder of filled.matchAll(/\{\{(source|tools) ([^}]+)\}\}/g)) {
    const paths = placeholder[2].trim().split(/\s+/)
    const block = placeholder[1] === 'source' ? await sourceBlock(paths[0]) : await toolList(paths)
    filled = filled.replace(placeholder[0], () => block)
  }
  return filled.trim() + '\n'
}

async function checkHeadings(markdown, pagePaths) {
  const sources = (await Promise.all(pagePaths.map(path => readText(new URL(path, siteRoot)))))
    .join('\n')
    .replace(/<[^>]+>/g, '')
    .replace(/\s+/g, ' ')
  for (const heading of markdown.matchAll(/^#{1,3} (.+)$/gm)) {
    if (!sources.includes(heading[1])) drift.push('heading "' + heading[1] + '" is not in ' + pagePaths.join(', '))
  }
}

async function checkTopicLists(loaded) {
  const docsTool = await readText(new URL('routes/_components/docs-tool/docs-tool.html', siteRoot))
  const agentsPage = await readText(new URL('routes/agents.html', siteRoot))
  const options = [...docsTool.matchAll(/<option value="([^"]+)"/g)].map(option => option[1])
  if (options.join(' ') !== loaded.map(topic => topic.id).join(' ')) drift.push('the options of docs-tool.html differ from the topics: ' + options.join(' '))
  for (const topic of loaded) {
    if (!agentsPage.includes('/docs/' + topic.id + '.md')) drift.push('routes/agents.html does not link /docs/' + topic.id + '.md')
  }
}

async function writePublic(path, content) {
  const target = new URL(path, publicRoot)
  await mkdir(new URL('./', target), { recursive: true })
  await writeFile(target, content)
  process.stdout.write('public/' + path + '  ' + (Buffer.byteLength(content) / 1000).toFixed(1) + ' kB\n')
}

const sourceExtensions = /\.(?:html|js|css|json|xml|txt)$/

async function collectSources(relativeDir, out) {
  const entries = await readdir(new URL(relativeDir + '/', siteRoot), { withFileTypes: true })
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    const path = relativeDir + '/' + entry.name
    if (entry.isDirectory()) {
      await collectSources(path, out)
    } else if (sourceExtensions.test(entry.name)) {
      out[path] = await readText(new URL(path, siteRoot))
    }
  }
}

async function generateSources() {
  const out = {}
  await collectSources('routes', out)
  await collectSources('lib', out)
  for (const path of ['pragma.json', 'package.json', 'public/robots.txt', 'public/sitemap.xml', 'utils/highlight.js', 'utils/code-lines.js']) {
    out[path] = await readText(new URL(path, siteRoot))
  }
  const code = JSON.stringify(out) + '\n'
  await writeFile(new URL('utils/sources.json', siteRoot), code)
  process.stdout.write('utils/sources.json  ' + (Buffer.byteLength(code) / 1000).toFixed(1) + ' kB\n')
}

async function generate() {
  await generateSources()
  const loaded = await loadTopics()
  for (const topic of loaded) await writePublic('docs/' + topic.id + '.md', topic.markdown + '\n')
  await writePublic('llms.txt', await fill(await readText(new URL('llms.md', templates)), loaded))
  await writePublic('llms-full.txt', await fill(await readText(new URL('llms-full.md', templates)), loaded))
  const landing = await fill(await readText(new URL('index.md', templates)), loaded)
  await writePublic('index.md', landing)
  const agents = await fill(await readText(new URL('agents.md', templates)), loaded)
  await writePublic('agents.md', agents)
  await checkHeadings(landing, ['routes/index.html', 'routes/_components/hero/hero.html', 'routes/_components/setup-tool/setup-tool.html', 'routes/_components/docs-tool/docs-tool.html'])
  await checkHeadings(agents, ['routes/agents.html', 'routes/_components/setup-tool/setup-tool.html', 'routes/_components/docs-tool/docs-tool.html'])
  await checkTopicLists(loaded)
  for (const problem of drift) process.stderr.write('drift: ' + problem + '\n')
  if (drift.length) process.exitCode = 1
}

generate()
