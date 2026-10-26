// Run the real wasm lab on desktop and touch-sized Chromium surfaces.
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile, mkdir } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import assert from 'node:assert/strict';
const [root, captures = 'target/particle-lab-captures'] = process.argv.slice(2);
if (!root) throw new Error('usage: particle-lab.mjs <site-dir> [capture-dir]');
await mkdir(captures, { recursive: true });
const types = { '.html':'text/html', '.js':'text/javascript', '.wasm':'application/wasm' };
const server = createServer(async (request, response) => {
  const path = normalize(decodeURIComponent(new URL(request.url, 'http://x').pathname)).replace(/^(\.\.[/\\])+/, '');
  try { const body = await readFile(join(root, path.endsWith('/') ? `${path}index.html` : path)); response.writeHead(200, {'content-type':types[extname(path)] ?? 'text/html'}); response.end(body); }
  catch { response.writeHead(404).end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM || undefined, args:['--enable-unsafe-webgpu','--use-angle=swiftshader'] });
try {
  for (const [name, device] of [['desktop',{ viewport:{width:1280,height:900} }], ['phone',{ viewport:{width:390,height:844},isMobile:true,hasTouch:true }]]) {
    const context = await browser.newContext(device);
    const page = await context.newPage();
    const problems = [];
    page.on('pageerror', e => problems.push(String(e)));
    page.on('console', e => { if(e.type() === 'error') problems.push(e.text()); });
    await page.goto(`http://127.0.0.1:${server.address().port}/`);
    await page.waitForTimeout(3000);
    const canvas = page.locator('#particle-lab');
    await canvas.click({position:{x:80,y:120}});
    await page.locator('#texture-file').setInputFiles({name:'pixel.png',mimeType:'image/png',buffer:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAADklEQVR4nGP4DwUMMAYAj4IP8TylVlEAAAAASUVORK5CYII=', 'base64')});
    await page.waitForTimeout(500);
    const shot = await page.screenshot({path:join(captures,`particle-lab-${name}.png`)});
    const count = await page.evaluate(async png => { const image=await createImageBitmap(new Blob([new Uint8Array(png)],{type:'image/png'})); const c=new OffscreenCanvas(image.width,image.height); const d=c.getContext('2d'); d.drawImage(image,0,0); const pixels=d.getImageData(0,0,image.width,image.height).data; const colors=new Set(); for(let i=0;i<pixels.length;i+=388) colors.add(pixels[i]<<16|pixels[i+1]<<8|pixels[i+2]); return colors.size; }, [...shot]);
    assert.ok(count > 20, `particle lab ${name} painted only ${count} colors`);
    assert.deepEqual(problems, []);
    await context.close();
  }
} finally { await browser.close(); server.close(); }
