// Bildschirmfotos der Hauptseiten bei 375 px und 1280 px (hell und dunkel)
// aus der Beispielakte; prüft nebenbei, dass keine Seite seitwärts scrollt.
//
//   node tools/screenshots.mjs [Ausgabeordner]
//
// Voraussetzungen: `npm install playwright` (im Ordner tools/ oder global) und
// ein vorinstalliertes Chromium (PLAYWRIGHT_BROWSERS_PATH). Der Server wird
// mit einer temporären Datenbank im Entwicklungsmodus gestartet.
import { chromium } from 'playwright';
import { spawn, execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const out = process.argv[2] ?? join(tmpdir(), 'vereinsakte-screens');
mkdirSync(out, { recursive: true });
const dir = mkdtempSync(join(tmpdir(), 'vereinsakte-'));
const port = 18090;
const config = join(dir, 'v.toml');
import { writeFileSync } from 'node:fs';
writeFileSync(config, `mode = "development"\nport = ${port}\nbase_url = "http://localhost:${port}"\ndatabase = "${join(dir, 'a.db')}"\n`);

const bin = process.env.VEREINSAKTE_BIN ?? 'target/debug/vereinsakte';
const run = (...args) => execFileSync(bin, ['-c', config, ...args], { stdio: 'inherit' });
run('migrate');
run('import', 'crates/vereinsakte-core/testdata/vereinsakte_beispiel.json');
const server = spawn(bin, ['-c', config, 'serve'], { stdio: 'ignore' });
await new Promise((r) => setTimeout(r, 1500));

// Mitglied -> Seiten, die für dieses Mitglied fotografiert werden.
const pages = JSON.parse(process.env.SCREEN_PAGES ?? '[["105","/"],["103","/meine-daten"],["105","/mitglieder"],["107","/mitglieder"]]');
const sizes = [['mobil', 375, 800], ['desktop', 1280, 900]];
const schemes = ['light', 'dark'];
let failed = false;
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM ?? undefined });
try {
  for (const [name, width, height] of sizes) {
    for (const scheme of schemes) {
      const context = await browser.newContext({ viewport: { width, height }, colorScheme: scheme });
      const page = await context.newPage();
      for (const [member, path] of pages) {
        await page.goto(`http://localhost:${port}/dev/login?next=${encodeURIComponent(path)}`);
        await page.locator(`input[name=member][value="${member}"]`).locator('xpath=ancestor::form').locator('button').click();
        await page.goto(`http://localhost:${port}${path}`);
        const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
        const file = join(out, `${name}-${scheme}-${member}${path.replace(/\W+/g, '_')}.png`);
        await page.screenshot({ path: file, fullPage: true });
        if (overflow > 0) { failed = true; console.error(`Seitwärts scrollbar (${overflow}px): ${name} ${scheme} ${path}`); }
      }
      await context.close();
    }
  }
} finally {
  await browser.close();
  server.kill();
}
console.log(`Bildschirmfotos in ${out}`);
process.exit(failed ? 1 : 0);
