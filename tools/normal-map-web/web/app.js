import init, { preset } from './pkg/sindri_normal_map_web.js';

const $ = id => document.getElementById(id);
const status = text => { $('status').textContent = text; };
let source = null, heightMap = null, normal = null, heightView = null, sheet = '', sourceName = 'texture.png', heightName = null;
let view = 'lit', lightX = .65, lightY = .3, orbitX = 8, orbitY = -16, drag = null, ready = false, workerReady = false, busy = false, queued = null, revision = 0, frame = 0;
const worker = new Worker(new URL('./worker.js', import.meta.url), {type:'module'});
const fields = ['method','strength','blur','detail','invert','alpha_height','edges','cells','green'];
function settings() {
  return Object.fromEntries(fields.map(id => [id, ['invert','alpha_height','cells'].includes(id) ? $(id).checked : ['strength','blur','detail'].includes(id) ? Number($(id).value) : $(id).value]));
}
function setSettings(data) {
  for (const id of fields) if (data[id] !== undefined) {
    if (['invert','alpha_height','cells'].includes(id)) $(id).checked = Boolean(data[id]);
    else $(id).value = data[id];
  }
  updateLabels();
}
function updateLabels() {
  for (const id of ['strength','blur','detail','light','ambient']) $(`${id}-value`).textContent = $(id).value;
  $('height-label').hidden = $('method').value !== 'height_map';
}
async function decode(file) {
  const bitmap = await createImageBitmap(file);
  if (bitmap.width * bitmap.height > 16777216) { bitmap.close(); throw new Error('Image is too large (maximum 16 megapixels).'); }
  const canvas = document.createElement('canvas'); canvas.width = bitmap.width; canvas.height = bitmap.height;
  const context = canvas.getContext('2d', { willReadFrequently:true }); context.drawImage(bitmap, 0, 0); bitmap.close();
  return context.getImageData(0, 0, canvas.width, canvas.height);
}
async function openSource(file) {
  if (!file) return;
  try { const image = await decode(file); source = image; sourceName = file.name; normal = null; $('empty').hidden = true; status(`${file.name} · ${image.width} × ${image.height}`); regenerate(); }
  catch (error) { status(`Could not open image: ${error.message || error}`); }
}
async function openHeight(file) {
  if (!file) return;
  try { heightMap = await decode(file); heightName = file.name; regenerate(); }
  catch (error) { status(`Could not open height map: ${error.message || error}`); }
}
worker.onmessage = event => {
  const message = event.data;
  if (message.ready) { workerReady = true; regenerate(); return; }
  if (message.fatal) { status(`Could not start generator: ${message.fatal}`); return; }
  busy = false;
  if (message.id === revision && source) {
    if (message.error) { normal = null; status(message.error); draw(); }
    else {
      normal = new ImageData(new Uint8ClampedArray(message.normal),source.width,source.height);
      heightView = message.heightView;
      $('download').disabled=false; $('save-settings').disabled=false;
      status(`${sourceName} · ${source.width} × ${source.height}`); draw();
    }
  }
  sendNext();
};
worker.onerror = event => { busy=false; status(`Generator error: ${event.message}`); };
function sendNext() {
  if (busy || !queued || !workerReady) return;
  busy=true; const job=queued; queued=null; worker.postMessage(job);
}
function regenerate() {
  if (!source || !ready) return;
  cancelAnimationFrame(frame);
  const requested = ++revision;
  frame = requestAnimationFrame(() => {
    $('download').disabled=true; $('save-settings').disabled=true;
    if ($('method').value === 'height_map' && (!heightMap || heightMap.width !== source.width || heightMap.height !== source.height)) {
      queued=null; normal=null; status('Choose a height map with the same dimensions as the source image.'); draw(); return;
    }
    normal=null; heightView=null; status('Generating normal map…');
    queued={id:requested,source:source.data,height:$('method').value === 'height_map' ? heightMap.data : new Uint8Array(),width:source.width,heightPx:source.height,settings:JSON.stringify(settings()),sheet,wantHeight:$('channel').value==='height'};
    sendNext();
  });
}
function draw() {
  if (!source) return;
  const w=source.width, h=source.height, scale=Math.min(1,640/Math.max(w,h)), pw=Math.max(1,Math.round(w*scale)), ph=Math.max(1,Math.round(h*scale));
  const out=new ImageData(pw,ph), light=Number($('light').value), ambient=Number($('ambient').value), axis=$('green').value==='down' ? -1 : 1;
  for (let y=0;y<ph;y++) for(let x=0;x<pw;x++) {
    const sx=Math.min(w-1,Math.floor(x*w/pw)), sy=Math.min(h-1,Math.floor(y*h/ph)), i=(sy*w+sx)*4, j=(y*pw+x)*4;
    const isNormal=view==='normal', isTexture=view==='texture' || (view==='split' && x<pw/2);
    if (isTexture || !normal) { out.data.set(source.data.subarray(i,i+4),j); continue; }
    if (isNormal) {
      const channel=$('channel').value, n=channel==='height' && heightView ? heightView[sy*w+sx] : channel==='red' ? normal.data[i] : channel==='green' ? normal.data[i+1] : normal.data[i+2];
      if (channel==='rgb') out.data.set(normal.data.subarray(i,i+4),j); else out.data.set([n,n,n,normal.data[i+3]],j);
      continue;
    }
    const nx=normal.data[i]/127.5-1, ny=(normal.data[i+1]/127.5-1)*axis, nz=normal.data[i+2]/127.5-1;
    const dx=(lightX-x/pw)*2, dy=(y/ph-lightY)*2, length=Math.hypot(dx,dy,light), shade=Math.max(0,(nx*dx+ny*dy+nz*light)/length), factor=Math.min(1,ambient+(1-ambient)*shade);
    out.data.set([source.data[i]*factor,source.data[i+1]*factor,source.data[i+2]*factor,source.data[i+3]],j);
  }
  const canvas=$('preview'); canvas.width=pw; canvas.height=ph; canvas.getContext('2d').putImageData(out,0,0);
}
function download(name, blob) { const link=document.createElement('a'); const url=URL.createObjectURL(blob); link.href=url; link.download=name; document.body.append(link); link.click(); link.remove(); setTimeout(()=>URL.revokeObjectURL(url),1000); }
$('download').addEventListener('click',()=>{ if (!normal) return; const canvas=document.createElement('canvas'); canvas.width=normal.width; canvas.height=normal.height; canvas.getContext('2d').putImageData(normal,0,0); canvas.toBlob(blob=>{ if(blob) download(`${sourceName.replace(/\.[^.]+$/, '')}_n.png`,blob); },'image/png'); });
$('save-settings').addEventListener('click',()=>{ const doc={format_version:1,source:sourceName,settings:settings()}; if(doc.settings.method==='height_map') doc.height_map=heightName; download(`${sourceName.replace(/\.[^.]+$/, '')}_n.normalmap`,new Blob([JSON.stringify(doc,null,2)+'\n'],{type:'application/json'})); });
$('load-settings').addEventListener('change',async e=>{ try { const doc=JSON.parse(await e.target.files[0].text()); if(doc.format_version!==1 || !doc.settings) throw new Error('Unsupported settings file.'); setSettings(doc.settings); status(`Settings loaded; choose ${doc.source}${doc.height_map ? ` and ${doc.height_map}` : ''}.`); regenerate(); } catch(error) { status(`Could not load settings: ${error.message || error}`); } });
for(const id of ['source','source-empty']) $(id).addEventListener('change',e=>openSource(e.target.files[0]));
$('height').addEventListener('change',e=>openHeight(e.target.files[0]));
$('sheet').addEventListener('change',async e=>{try{ sheet=await e.target.files[0].text(); $('sheet-name').textContent=e.target.files[0].name; $('clear-sheet').disabled=false; regenerate(); }catch(error){status(String(error));}});
for(const id of fields) $(id).addEventListener('input',()=>{ updateLabels(); regenerate(); });
for(const id of ['light','ambient']) $(id).addEventListener('input',()=>{updateLabels();draw();});
$('channel').addEventListener('input',()=>{if($('channel').value==='height' && !heightView) regenerate(); else draw();});
$('clear-sheet').addEventListener('click',()=>{sheet='';$('sheet').value='';$('sheet-name').textContent='No sheet loaded';$('clear-sheet').disabled=true;regenerate();});
$('preset').addEventListener('change',()=>{try{setSettings(JSON.parse(preset($('preset').selectedIndex,JSON.stringify(settings()))));regenerate();}catch(error){status(String(error));}});
document.querySelectorAll('[data-view]').forEach(button=>button.addEventListener('click',()=>{view=button.dataset.view; $('preview').style.transform=view==='inspect' ? `perspective(900px) rotateX(${orbitX}deg) rotateY(${orbitY}deg) scale(.8)` : ''; document.querySelectorAll('[data-view]').forEach(b=>b.classList.toggle('selected',b===button));draw();}));
$('preview').addEventListener('pointerdown',e=>{e.currentTarget.setPointerCapture(e.pointerId);drag={x:e.clientX,y:e.clientY};if(view!=='inspect')moveLight(e);});
$('preview').addEventListener('pointermove',e=>{if(!e.buttons)return;if(view==='inspect' && drag){orbitY=Math.max(-70,Math.min(70,orbitY+(e.clientX-drag.x)*.4));orbitX=Math.max(-70,Math.min(70,orbitX-(e.clientY-drag.y)*.4));drag={x:e.clientX,y:e.clientY};$('preview').style.transform=`perspective(900px) rotateX(${orbitX}deg) rotateY(${orbitY}deg) scale(.8)`;}else moveLight(e);});
$('preview').addEventListener('pointerup',()=>{drag=null;});
function moveLight(event){const canvas=$('preview'), rect=canvas.getBoundingClientRect(), aspect=canvas.width/canvas.height; const width=Math.min(rect.width,rect.height*aspect), height=Math.min(rect.height,rect.width/aspect), left=rect.left+(rect.width-width)/2, top=rect.top+(rect.height-height)/2; lightX=Math.max(0,Math.min(1,(event.clientX-left)/width)); lightY=Math.max(0,Math.min(1,(event.clientY-top)/height)); draw();}
$('stage').addEventListener('dragover',e=>e.preventDefault()); $('stage').addEventListener('drop',e=>{e.preventDefault();openSource(e.dataTransfer.files[0]);});
try { await init(); ready=true; status('Choose an image to begin.'); regenerate(); } catch(error) { status(`Could not start normal-map generator: ${error}`); }
