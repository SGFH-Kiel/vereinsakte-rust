// Prüft das Ziehen auf dem Aufgabenboard: Karte nach WIP ziehen, neu laden, Status bleibt.
//   node tools/board-test.mjs
import { chromium } from 'playwright';
import { spawn, execFileSync } from 'node:child_process';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const dir = mkdtempSync(join(tmpdir(), 'vereinsakte-board-'));
const port = 18091;
const config = join(dir, 'v.toml');
writeFileSync(config, `mode = "development"\nport = ${port}\nbase_url = "http://localhost:${port}"\ndatabase = "${join(dir, 'a.db')}"\n`);
const bin = process.env.VEREINSAKTE_BIN ?? 'target/debug/vereinsakte';
const run = (...args) => execFileSync(bin, ['-c', config, ...args], { stdio: 'ignore' });
run('migrate');
run('import', 'crates/vereinsakte-core/testdata/vereinsakte_beispiel.json');
const server = spawn(bin, ['-c', config, 'serve'], { stdio: 'ignore' });
await new Promise((r) => setTimeout(r, 1500));
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM ?? undefined });
let ok = false;
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
  await page.goto(`http://localhost:${port}/dev/login?next=${encodeURIComponent('/aufgaben')}`);
  await page.locator('input[name=member][value="102"]').locator('xpath=ancestor::form').locator('button').click();
  await page.goto(`http://localhost:${port}/aufgaben`);
  const card = page.locator('.task-card', { hasText: 'Segeltuch' });
  const target = page.locator('[data-status="in_progress"]');
  await card.dragTo(target);
  await page.waitForLoadState('networkidle');
  await page.reload();
  const inWip = await page.locator('[data-status="in_progress"] .task-card', { hasText: 'Segeltuch' }).count();
  console.log(inWip === 1 ? 'OK: Karte liegt nach dem Ziehen in WIP' : 'FEHLER: Karte nicht in WIP');
  ok = inWip === 1;
} finally {
  await browser.close();
  server.kill();
}
process.exit(ok ? 0 : 1);
