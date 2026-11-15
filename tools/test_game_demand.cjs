#!/usr/bin/env node
// Built Bevy game, not a stub: menu IO, scene entry, and cold offline battle.
const {chromium} = require('playwright');
const fs = require('node:fs/promises');
const path = require('node:path');
const os = require('node:os');
const assert = require('node:assert/strict');
(async () => {
  const base = process.env.PWA_GAME_URL || 'http://localhost:8767/';
  const output = path.resolve(process.env.PWA_GAME_OUTPUT || 'target/pwa-real-demand');
  await fs.mkdir(output, {recursive:true});
  const manifest = await (await fetch(base+'asset-manifest.json')).json();
  const logical = new Map(manifest.files.flatMap(f=>[[new URL(f.path,base).href,f],[new URL(f.url,base).href,f]]));
  const profile = await fs.mkdtemp(path.join(os.tmpdir(),'rogue-real-demand-'));
  const launch = () => chromium.launchPersistentContext(profile, {
    executablePath:process.env.CHROMIUM_EXECUTABLE_PATH, headless:true,
    viewport:{width:1280,height:720},args:['--no-sandbox','--disable-dev-shm-usage','--use-gl=angle','--use-angle=swiftshader','--enable-unsafe-swiftshader'],
  });
  let context, page;
  const requests=[],errors=[],messages=[],failedRequests=[];
  const attach = () => {
    context.on('requestfailed',request=>failedRequests.push({url:request.url(),error:request.failure()?.errorText}));
    context.on('request',request=>{const file=logical.get(request.url()); if(file)requests.push({path:file.path,group:file.group,url:request.url()});});
    context.on('page',page=>{
      page.on('crash',()=>errors.push('Browser renderer crashed'));
      page.on('pageerror',e=>errors.push(String(e)));
      page.on('console',msg=>{messages.push(msg.type()+': '+msg.text());if((msg.type()==='error'||/(?:%c|\b)ERROR(?:%c|\b)/.test(msg.text()))&&!msg.text().includes('exceptions for control flow'))errors.push(msg.text());});
    });
  };
  const status = () => page.evaluate(async()=>{
    const reg=await navigator.serviceWorker.getRegistration();
    return new Promise(resolve=>{const channel=new MessageChannel();channel.port1.onmessage=e=>{channel.port1.close();resolve(e.data);};reg.active.postMessage({type:'STATUS',target:'client',groups:['all'],wasmEncoding:'gzip'},[channel.port2]);});
  });
  async function menu() {
    await page.addInitScript(() => {
      const original = ServiceWorker.prototype.postMessage;
      ServiceWorker.prototype.postMessage = function (message, ...args) {
        console.debug('Cache request', JSON.stringify(message));
        return original.call(this, message, ...args);
      };
    });
    await page.goto(base+'?lang=en');
    await page.locator('#start').click();
    await page.waitForFunction(()=>document.body.classList.contains('playing'),null,{timeout:180000});
    await page.waitForTimeout(30000);
  }
  async function battleScreenshot(name) {
    const file = path.join(output, name);
    // SwiftShader can warm HUD pipelines after world geometry. Require the actual
    // health/gate bars, rather than treating an engine-start flag as a battle pass.
    for (let attempt = 0; attempt < 15; attempt++) {
      await page.waitForTimeout(4000);
      await page.screenshot({path:file});
      const bytes = await fs.readFile(file);
      const visible = await page.evaluate(async data => {
        const bitmap = await createImageBitmap(await (await fetch(data)).blob());
        const canvas = new OffscreenCanvas(305, 130);
        const ctx = canvas.getContext('2d');
        ctx.drawImage(bitmap, 15, 60, 305, 130, 0, 0, 305, 130);
        const pixels = ctx.getImageData(0, 0, 305, 130).data;
        let green = 0;
        for (let i = 0; i < pixels.length; i += 4)
          if (pixels[i + 1] > 110 && pixels[i] < 100 && pixels[i + 2] < 190) green++;
        ctx.clearRect(0, 0, 305, 130);
        ctx.drawImage(bitmap, 650, 640, 200, 70, 0, 0, 200, 70);
        bitmap.close();
        const bottom = ctx.getImageData(0, 0, 200, 70).data;
        let gold = 0;
        for (let i = 0; i < bottom.length; i += 4)
          if (bottom[i] > 170 && bottom[i + 1] > 110 && bottom[i + 2] < 150) gold++;
        return green > 250 && gold > 50;
      }, 'data:image/png;base64,' + bytes.toString('base64'));
      if (visible) return;
    }
    throw Error('Battle HUD did not become visible: '+file);
  }
  try {
    context=await launch();attach();page=await context.newPage();
    await menu();
    await page.screenshot({path:path.join(output,'menu-before-download.png')});
    const first=await status();
    await fs.writeFile(path.join(output,'first-status.json'),JSON.stringify(first,null,2));
    if (!first.groups.find(g=>g.id==='core').ready) {
      const inventory = await page.evaluate(async () => {
        const result = {};
        for (const name of await caches.keys()) {
          const cache = await caches.open(name);
          result[name] = (await cache.keys()).map(request=>request.url);
        }
        return result;
      });
      await fs.writeFile(path.join(output,'cache-inventory.json'),JSON.stringify(inventory,null,2));
    }
    assert.equal(first.groups.find(g=>g.id==='core').ready,true);
    assert.ok(first.groups.find(g=>g.id==='combat').downloadBytes>0);
    assert.equal(requests.filter(r=>r.group==='combat').length,0,'menu must not request battle assets');
    console.log('PASS actual Bevy menu made zero battle-asset requests');
    await page.mouse.click(285,220);
    await page.locator('#downloads-dialog').waitFor({state:'visible',timeout:20000});
    await page.screenshot({path:path.join(output,'chapter-download.png')});
    await page.locator('#downloads-confirm').click();
    await page.locator('#downloads-dialog').waitFor({state:'hidden',timeout:180000});
    await page.waitForTimeout(30000);
    await page.screenshot({path:path.join(output,'recruitment-online.png')});
    const prepared=await status();
    assert.equal(prepared.groups.find(g=>g.id==='combat').ready,true);
    await page.mouse.click(480,450); // First-run guide: public Skip button.
    await page.waitForTimeout(1000);
    await page.mouse.click(200,300);
    await battleScreenshot('battle-online.png');
    console.log('PASS chapter prepares complete shared battle pack before recruitment');
    await page.mouse.click(640,32);
    await page.waitForTimeout(3000);
    await page.screenshot({path:path.join(output,'settings-entry.png')});
    await page.mouse.click(640,514);
    await page.locator('#downloads-dialog').waitFor({state:'visible',timeout:15000});
    await page.waitForTimeout(1000);
    await page.screenshot({path:path.join(output,'settings-downloads.png')});
    await page.locator('#downloads-close').click();
    await page.keyboard.press('Escape');
    console.log('PASS actual game settings opens and closes the downloads manager');
    await context.close();
    context=await launch();await context.setOffline(true);attach();page=await context.newPage();
    await menu();
    await page.mouse.click(285,220);
    await page.waitForTimeout(30000);
    assert.equal(await page.locator('#downloads-dialog').isVisible(),false);
    await page.screenshot({path:path.join(output,'recruitment-offline.png')});
    await page.mouse.click(200,300);
    await battleScreenshot('battle-offline.png');
    const offline=await status();
    assert.equal(offline.groups.find(g=>g.id==='combat').ready,true);
    assert.deepEqual(failedRequests.filter(r=>!new URL(r.url).pathname.endsWith('/sw.js')),[]);
    assert.deepEqual(errors.filter(e=>!e.includes('ERR_INTERNET_DISCONNECTED')),[]);
    await fs.writeFile(path.join(output,'results.json'),JSON.stringify({first,prepared,offline,requests,failedRequests,errors},null,2));
    console.log('PASS cold browser restart offline enters a prepared expedition; screenshots require visual review');
  } finally {
    await fs.writeFile(path.join(output,'console.log'),messages.join('\n'));
    await fs.writeFile(path.join(output,'errors.json'),JSON.stringify(errors,null,2));
    await context?.close();await fs.rm(profile,{recursive:true,force:true});
  }
})().catch(error=>{console.error(error);process.exitCode=1;});
