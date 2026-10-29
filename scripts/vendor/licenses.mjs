// Écrit ui/vendor/LICENSES.md : la licence de chaque paquet inclus dans le paquet esbuild.
// Usage : node licenses.mjs <metafile.json> <sortie.md>
import { readFileSync, writeFileSync, existsSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const [metafile, output] = process.argv.slice(2);
const meta = JSON.parse(readFileSync(metafile, 'utf8'));

const packages = new Set();
for (const input of Object.keys(meta.inputs)) {
  const match = input.match(/node_modules\/((?:@[^/]+\/)?[^/]+)\//);
  if (match) packages.add(match[1]);
}

const sections = [...packages].sort().map((name) => {
  const dir = join('node_modules', name);
  const pkg = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'));
  const file = readdirSync(dir).find((f) => /^licen[cs]e/i.test(f));
  const text = file && existsSync(join(dir, file)) ? readFileSync(join(dir, file), 'utf8').trim() : `Licence : ${pkg.license}`;
  return `## ${name} ${pkg.version} (${pkg.license})\n\n\`\`\`text\n${text}\n\`\`\``;
});

writeFileSync(
  output,
  `# Licences de ui/vendor/codemirror.js\n\nFichier généré par scripts/vendor-codemirror.sh, ne pas modifier.\n\n${sections.join('\n\n')}\n`,
);
