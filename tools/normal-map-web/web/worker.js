// Generation runs away from the UI thread, using the same Rust library as the editor.
import init, { generate_map, generate_height } from './pkg/sindri_normal_map_web.js';
try { await init(); self.postMessage({ready:true}); }
catch (error) { self.postMessage({fatal:String(error)}); }
self.onmessage = event => {
  const job = event.data;
  try {
    const normal = generate_map(job.source, job.height, job.width, job.heightPx, job.settings, job.sheet);
    const heightView = job.wantHeight ? generate_height(job.source, job.height, job.width, job.heightPx, job.settings, job.sheet) : null;
    self.postMessage({id:job.id, normal, heightView}, heightView ? [normal.buffer, heightView.buffer] : [normal.buffer]);
  } catch (error) { self.postMessage({id:job.id, error:String(error)}); }
};
