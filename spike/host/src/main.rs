use std::time::Instant;
use wasmtime::*;

fn main() -> anyhow_free::R {
    let path = std::env::args().nth(1).unwrap();
    let text = "This is an test of the the sentence. Their going to the store.";
    // native baseline
    let t0 = Instant::now(); let _ = plugin_spike::lint_json(text); println!("native cold call: {:?}", t0.elapsed());
    for i in 0..3 {
        let t = Instant::now();
        let s = plugin_spike::lint_json(text);
        println!("native call {i}: {:?} ({} bytes)", t.elapsed(), s.len());
    }
    // wasmtime
    let engine = Engine::default();
    let t = Instant::now();
    let module = Module::from_file(&engine, &path)?;
    println!("wasmtime compile: {:?}", t.elapsed());
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);
    linker.define_unknown_imports_as_traps(&module)?;
    let t = Instant::now();
    let inst = linker.instantiate(&mut store, &module)?;
    println!("wasmtime instantiate: {:?}", t.elapsed());
    let mem = inst.get_memory(&mut store, "memory").unwrap();
    let alloc = inst.get_typed_func::<u32, u32>(&mut store, "harper_alloc")?;
    let free = inst.get_typed_func::<(u32, u32), ()>(&mut store, "harper_free")?;
    let call = inst.get_typed_func::<(u32, u32), u64>(&mut store, "harper_call")?;
    for i in 0..3 {
        let t = Instant::now();
        let p = alloc.call(&mut store, text.len() as u32)?;
        mem.write(&mut store, p as usize, text.as_bytes())?;
        let r = call.call(&mut store, (p, text.len() as u32))?;
        let (ptr, len) = ((r >> 32) as u32, (r & 0xffff_ffff) as u32);
        let mut buf = vec![0u8; len as usize];
        mem.read(&store, ptr as usize, &mut buf)?;
        free.call(&mut store, (ptr, len))?;
        free.call(&mut store, (p, text.len() as u32))?;
        println!("wasmtime call {i}: {:?} ({} bytes)", t.elapsed(), buf.len());
    }
    Ok(())
}
mod anyhow_free { pub type R = Result<(), wasmtime::Error>; }
