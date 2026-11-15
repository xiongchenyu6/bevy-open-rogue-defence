#!/usr/bin/env node
// Real production startup UI and SW, with only the heavyweight Bevy engine stubbed.
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const http = require('node:http');
const {spawnSync} = require('node:child_process');
const {gzipSync} = require('node:zlib');
const {chromium} = require('playwright');
const ROOT = path.resolve(__dirname, '..');
const OUTPUT = path.resolve(process.env.PWA_TEST_OUTPUT || 'target/pwa-update-ui-tests');
const results = [];
const pass = text => { results.push(text); console.log(`PASS ${text}`); };

async function main() {
  await fs.mkdir(OUTPUT, {recursive: true});
  const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'rogue-pwa-ui-'));
  const fixtures = {};
  for (const revision of ['v1', 'v2', 'v3']) {
    const directory = path.join(temporary, revision);
    await fs.cp(path.join(ROOT, 'web'), directory, {recursive: true});
    await fs.mkdir(path.join(directory, 'assets/fonts'), {recursive: true});
    await fs.copyFile(path.join(ROOT, 'assets/fonts/rogue_commander_ui.ttf'), path.join(directory, 'assets/fonts/rogue_commander_ui.ttf'));
    await fs.writeFile(path.join(directory, 'assets/changed.txt'), revision);
    await fs.writeFile(path.join(directory, 'rogue_commander.js'), `export default async function init(options) { if (!options?.module_or_path) throw Error('Missing prepared WASM'); window.fixtureEngineVersion = '${revision}'; }`);
    const wasm = Buffer.from([0,97,115,109,1,0,0,0]);
    await fs.writeFile(path.join(directory, 'rogue_commander_bg.wasm'), wasm);
    await fs.writeFile(path.join(directory, 'rogue_commander_bg.wasm.gz'), gzipSync(wasm));
    const generated = spawnSync('python3', ['tools/generate_web_manifest.py', directory, '--revision', revision, '--template', 'web/sw.js'], {cwd: ROOT, encoding: 'utf8'});
    assert.equal(generated.status, 0, generated.stderr);
    fixtures[revision] = directory;
  }
  let current = 'v1', failResource = false, legacyMode = false;
  const requests = [];
  const server = http.createServer(async (request, response) => {
    const url = new URL(request.url, 'http://fixture');
    const relative = url.pathname === '/game/' ? 'index.html' : decodeURIComponent(url.pathname.slice('/game/'.length));
    requests.push({revision: current, path: relative});
    response.setHeader('Cache-Control', 'no-store');
    response.setHeader('Service-Worker-Allowed', '/game/');
    if (!url.pathname.startsWith('/game/') || relative.includes('..')) { response.writeHead(404).end(); return; }
    if (failResource && relative === 'assets/changed.txt') { response.writeHead(503).end(); return; }
    if (legacyMode && relative === 'sw.js') {
      response.setHeader('Content-Type', 'text/javascript');
      response.end("self.addEventListener('install',()=>self.skipWaiting()); self.addEventListener('activate',event=>event.waitUntil(self.clients.claim())); self.addEventListener('fetch',event=>{if(event.request.mode==='navigate')event.respondWith(fetch(event.request));});"); return;
    }
    if (legacyMode && relative === 'index.html') {
      response.setHeader('Content-Type', 'text/html');
      response.end('<!doctype html><h1>Legacy network-only game</h1>'); return;
    }
    try {
      const body = await fs.readFile(path.join(fixtures[current], relative));
      const type = relative.endsWith('.js') ? 'text/javascript' : relative.endsWith('.html') ? 'text/html' : relative.endsWith('.css') ? 'text/css' : relative.endsWith('.png') ? 'image/png' : relative.endsWith('.ttf') ? 'font/ttf' : relative.endsWith('.webmanifest') ? 'application/manifest+json' : 'application/octet-stream';
      response.setHeader('Content-Type', type);
      response.writeHead(200).end(body);
    } catch { response.writeHead(404).end(); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const base = `http://127.0.0.1:${server.address().port}/game/?lang=en`;
  const launch = (profile = 'profile') => chromium.launchPersistentContext(path.join(temporary, profile), {
    headless: true, executablePath: process.env.CHROMIUM_EXECUTABLE_PATH || undefined,
    viewport: {width: 1280, height: 720}, serviceWorkers: 'allow',
  });
  let context;
  const errors = [];
  const observe = page => page.on('pageerror', error => errors.push(String(error)));
  try {
    context = await launch();
    let page = await context.newPage(); observe(page);
    await page.goto(base);
    await page.locator('#cache-status').filter({hasText: 'Resources are saved on first play'}).waitFor();
    await page.locator('#start').click();
    await page.waitForFunction(() => document.body.classList.contains('playing'));
    assert.equal(await page.evaluate(() => window.fixtureEngineVersion), 'v1');
    assert.equal(requests.filter(r => r.path === 'rogue_commander_bg.wasm.gz').length, 1);
    assert.equal(requests.filter(r => r.path === 'rogue_commander_bg.wasm').length, 0);
    pass('actual Start button prepares resources before engine initialization');

    requests.length = 0;
    await page.reload();
    await page.locator('#cache-status').filter({hasText: 'Ready to play offline'}).waitFor();
    await page.locator('#start').click();
    await page.waitForFunction(() => document.body.classList.contains('playing'));
    assert.equal(await page.evaluate(() => window.fixtureEngineVersion), 'v1');
    assert.equal(requests.filter(r => r.path !== 'sw.js').length, 0, JSON.stringify(requests));
    pass('actual second Start initializes from saved resources with zero asset downloads');

    current = 'v2';
    await page.reload();
    await page.evaluate(async () => (await navigator.serviceWorker.getRegistration()).update());
    await page.locator('#cache-update').waitFor({state: 'visible'});
    for (const [width, height] of [[1280,720], [844,390], [393,852]]) {
      await page.setViewportSize({width, height});
      for (const selector of ['#start', '#cache-update', '#cache-later']) {
        const box = await page.locator(selector).boundingBox();
        assert.ok(box && box.x >= 0 && box.y >= 0 && box.x + box.width <= width + 1 && box.y + box.height <= height + 1, `${selector} clipped at ${width}x${height}: ${JSON.stringify(box)}`);
      }
      await page.screenshot({path: path.join(OUTPUT, `update-offer-${width}x${height}.png`)});
    }
    pass('update offer and both actions fit desktop, landscape phone and portrait viewports');
    await page.locator('#cache-later').click();
    await page.locator('#cache-update').waitFor({state: 'hidden'});
    await page.locator('#start').click();
    await page.waitForFunction(() => document.body.classList.contains('playing'));
    assert.equal(await page.evaluate(() => window.fixtureEngineVersion), 'v1');
    pass('Later starts the old engine instead of silently upgrading');

    await context.close();
    context = await launch();
    page = await context.newPage(); observe(page);
    await page.goto(base);
    await page.locator('#cache-update').waitFor({state: 'visible'});
    assert.match(await page.locator('#cache-status').textContent(), /new version/);
    pass('update offer survives a full browser restart and natural worker activation');
    await page.locator('#cache-update').click();
    await page.waitForFunction(() => document.querySelector('#cache-status').textContent.includes('Ready to play offline'));
    await page.locator('#start').click();
    await page.waitForFunction(() => document.body.classList.contains('playing'));
    assert.equal(await page.evaluate(() => window.fixtureEngineVersion), 'v2');
    pass('Download & update commits, reloads and starts the new engine');

    current = 'v3'; failResource = true;
    await page.reload();
    await page.evaluate(async () => (await navigator.serviceWorker.getRegistration()).update());
    await page.locator('#cache-update').waitFor({state: 'visible'});
    await page.locator('#cache-update').click();
    await page.locator('#cache-status').filter({hasText: 'Update failed'}).waitFor();
    assert.equal(await page.locator('#start').isEnabled(), true);
    assert.match(await page.locator('#cache-update').textContent(), /Retry update/);
    await page.screenshot({path: path.join(OUTPUT, 'failed-update-old-version-available.png')});
    await page.locator('#start').click();
    await page.waitForFunction(() => document.body.classList.contains('playing'));
    assert.equal(await page.evaluate(() => window.fixtureEngineVersion), 'v2');
    pass('failed update shows retry while keeping Start available for previous engine');

    await context.close();
    current = 'v1'; failResource = false; legacyMode = true;
    context = await launch('legacy-profile');
    page = await context.newPage(); observe(page);
    await page.goto(base);
    await page.evaluate(async () => {
      await navigator.serviceWorker.register('./sw.js', {scope: './', updateViaCache: 'none'});
      await navigator.serviceWorker.ready;
    });
    await page.waitForFunction(() => Boolean(navigator.serviceWorker.controller));
    const legacyTab = await context.newPage(); observe(legacyTab);
    await legacyTab.goto(base);
    await legacyTab.waitForFunction(() => Boolean(navigator.serviceWorker.controller));
    legacyMode = false;
    await page.reload();
    await page.locator('#start').waitFor();
    await page.waitForFunction(async () => Boolean((await navigator.serviceWorker.getRegistration()).waiting));
    let migrations = 0;
    page.on('request', request => { if (request.isNavigationRequest() && request.frame() === page.mainFrame()) migrations++; });
    await page.locator('#start').click();
    await page.waitForFunction(() => document.body.classList.contains('playing'), null, {timeout: 45000});
    assert.equal(await page.evaluate(() => window.fixtureEngineVersion), 'v1');
    assert.equal(migrations, 1, `Unexpected migration reload count: ${migrations}`);
    assert.equal(await page.evaluate(() => sessionStorage.getItem('rogue-cache-resume-start')), null);
    pass('legacy network-only worker migrates through waiting worker and resumes Start after one reload');
    assert.deepEqual(errors, []);
    await fs.writeFile(path.join(OUTPUT, 'results.json'), JSON.stringify({passed: results, pageErrors: errors, engine: 'stub only; production HTML/CSS/PWA/SW and real CacheStorage'}, null, 2));
    console.log(`\n${results.length} real startup-UI scenarios passed; evidence: ${OUTPUT}`);
  } finally {
    await context?.close();
    await new Promise(resolve => server.close(resolve));
    await fs.rm(temporary, {recursive: true, force: true});
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
