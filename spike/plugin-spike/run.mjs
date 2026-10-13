import fs from 'node:fs';
const f = process.argv[2];
const text = process.argv[3] ?? "This is an test of the the sentence. Their going to the store.";
const mod = await WebAssembly.compile(fs.readFileSync(f));
const imps=WebAssembly.Module.imports(mod);console.log('imports:',imps.length);console.log('exports:',WebAssembly.Module.exports(mod).length);
const io={};for(const i of imps){(io[i.module]??={})[i.name]=()=>{throw new Error('stub called: '+i.name)};}
let t=performance.now();
const inst = await WebAssembly.instantiate(mod, io);
console.log('instantiate ms', (performance.now()-t).toFixed(1));
const x = inst.exports;
const enc = new TextEncoder().encode(text);
const run = () => {
  const p = x.harper_alloc(enc.length);
  new Uint8Array(x.memory.buffer, p, enc.length).set(enc);
  const r = x.harper_call(p, enc.length);
  const ptr = Number(r >> 32n), len = Number(r & 0xffffffffn);
  const s = new TextDecoder().decode(new Uint8Array(x.memory.buffer, ptr, len));
  x.harper_free(ptr, len); x.harper_free(p, enc.length);
  return s;
};
for (let i=0;i<3;i++){ t=performance.now(); const s=run(); console.log('call', i, (performance.now()-t).toFixed(1),'ms', i==0? s.slice(0,300):''); }
console.log('mem MB', x.memory.buffer.byteLength/1e6);
