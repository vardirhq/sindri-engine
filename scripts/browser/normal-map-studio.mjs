// Exercise normal-map creation and download on desktop and mobile.
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import assert from 'node:assert/strict';
const root = process.argv[2];
const captures = process.argv[3];
if (captures) await mkdir(captures, {recursive:true});
if (!root) throw new Error('usage: normal-map-studio.mjs <site-dir>');
const types = {'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.css':'text/css'};
const server=createServer(async(req,res)=>{const path=normalize(decodeURIComponent(new URL(req.url,'http://x').pathname)).replace(/^(\.\.[/\\])+/, '');try{const asset=path.endsWith('/')?`${path}index.html`:path;const bytes=await readFile(join(root,asset));res.writeHead(200,{'content-type':types[extname(asset)]??'text/plain'});res.end(bytes);}catch{res.writeHead(404).end();}});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const browser=await chromium.launch({executablePath:process.env.CHROMIUM||undefined});
try {
  for (const viewport of [{width:1280,height:800},{width:390,height:844}]) {
    const page=await browser.newPage({viewport,hasTouch:viewport.width<500});
    const errors=[]; page.on('pageerror',error=>errors.push(String(error)));
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    await page.waitForFunction(()=>document.querySelector('#status').textContent.includes('Choose an image'),null,{timeout:60000});
    const image=Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAGUlEQVR4nGP4DwQiIiL/GQICAv739PQEAABbYQkbCmOg+wAAAABJRU5ErkJggg==','base64');
    await page.locator('#source').setInputFiles({name:'fixture.png',mimeType:'image/png',buffer:image});
    await page.waitForFunction(()=>!document.querySelector('#download').disabled,null,{timeout:60000});
    assert.equal(await page.locator('#empty').isHidden(),true);
    await page.locator('[data-view="normal"]').click();
    await page.locator('#preset').selectOption({label:'Stone'});
    await page.waitForFunction(()=>!document.querySelector('#download').disabled);
    await page.locator('[data-view="inspect"]').click();
    assert.ok((await page.locator('#preview').getAttribute('style')).includes('perspective'));
    await page.locator('#method').selectOption('height_map');
    await page.waitForFunction(()=>document.querySelector('#download').disabled);
    await page.locator('#height').setInputFiles({name:'height.png',mimeType:'image/png',buffer:image});
    await page.waitForFunction(()=>!document.querySelector('#download').disabled);
    await page.locator('[data-view="normal"]').click();
    await page.locator('#channel').selectOption('height');
    await page.waitForFunction(()=>!document.querySelector('#download').disabled);
    const sheet=Buffer.from(JSON.stringify({format_version:1,grid:{columns:2,rows:1}}));
    await page.locator('#sheet').setInputFiles({name:'fixture.sheet',mimeType:'application/json',buffer:sheet});
    await page.waitForFunction(()=>document.querySelector('#sheet-name').textContent==='fixture.sheet');
    await page.waitForFunction(()=>!document.querySelector('#download').disabled);
    const downloadPromise=page.waitForEvent('download'); await page.locator('#download').click(); const download=await downloadPromise;
    assert.equal(download.suggestedFilename(),'fixture_n.png');
    const png = await readFile(await download.path());
    const alpha = await page.evaluate(async bytes=>{const bitmap=await createImageBitmap(new Blob([new Uint8Array(bytes)],{type:'image/png'}));const c=document.createElement('canvas');c.width=bitmap.width;c.height=bitmap.height;const ctx=c.getContext('2d');ctx.drawImage(bitmap,0,0);return [...ctx.getImageData(0,0,c.width,c.height).data].filter((_,i)=>i%4===3);}, [...png]);
    assert.deepEqual(alpha,[255,255,255,80]);
    const settingsDownload=page.waitForEvent('download'); await page.locator('#save-settings').click(); const saved=await settingsDownload;
    assert.equal(saved.suggestedFilename(),'fixture_n.normalmap');
    const document=JSON.parse(await readFile(await saved.path(),'utf8'));
    assert.equal(document.source,'fixture.png'); assert.equal(document.height_map,'height.png'); assert.equal(document.settings.method,'height_map');
    await page.locator('#load-settings').setInputFiles({name:'fixture_n.normalmap',mimeType:'application/json',buffer:Buffer.from(JSON.stringify(document))});
    await page.waitForFunction(()=>!document.querySelector('#download').disabled);
    assert.equal(await page.locator('#method').inputValue(),'height_map');
    if (captures) { await page.evaluate(()=>window.scrollTo(0,0)); await page.screenshot({path:join(captures,`normal-map-${viewport.width}.png`)}); }
    assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth <= window.innerWidth));
    assert.deepEqual(errors,[]);
    await page.close();
  }
} finally {await browser.close();server.close();}
