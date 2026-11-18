// Lint do dialeto: o código do app (fora de src/lib) não escreve no DOM
// diretamente; quem escreve é `h`/`list`, sempre dentro de effect.
// Uma linha pode se justificar com o comentário `// dom-ok: motivo`.
// A regra "não escrever em signal dentro de effect" é garantida pelo runtime.
import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';

const FORBIDDEN = [
  [/\.(innerHTML|outerHTML|textContent|innerText|className|hidden|disabled|checked|value)\s*=(?!=)/, 'atribuição direta a propriedade do DOM'],
  [/\.(dataset|style)\.[\w-]+\s*=(?!=)/, 'atribuição direta a dataset/style'],
  [/\.classList\.(add|remove|toggle|replace)\(/, 'classList'],
  [/\.(setAttribute|removeAttribute|toggleAttribute)\(/, 'atributo escrito direto'],
  [/\.(append|prepend|appendChild|insertBefore|replaceChildren|replaceWith|insertAdjacent\w+|remove)\(/, 'árvore do DOM alterada direto'],
  [/document\.(createElement|querySelector|getElementById)/, 'use h() e referências em variáveis'],
  [/\.addEventListener\(/, "use a prop on* de h()"],
  [/\beffect\(\s*\(\)\s*=>\s*\w+\.(set|update)\(/, 'effect que só escreve em signal: use computed'],
];

async function* walk(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (path !== join('src', 'lib')) yield* walk(path);
    } else if (path.endsWith('.js')) yield path;
  }
}

let problems = 0;
for await (const file of walk('src')) {
  const lines = (await readFile(file, 'utf8')).split('\n');
  lines.forEach((line, index) => {
    if (line.includes('dom-ok:') || line.trim().startsWith('//')) return;
    for (const [pattern, message] of FORBIDDEN) {
      if (!pattern.test(line)) continue;
      problems++;
      console.log(`${file}:${index + 1}: ${message}\n    ${line.trim()}`);
    }
  });
}
console.log(problems ? `\n${problems} problema(s)` : 'check: nenhum problema');
process.exit(problems ? 1 : 0);
