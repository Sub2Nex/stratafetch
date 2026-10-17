// Run from repository root with Playwright installed; uses an isolated test profile only.
const {chromium}=require('playwright');
const path=require('path'),http=require('http'),assert=require('assert/strict');
(async()=>{
 const transfers=[];
 const server=http.createServer((req,res)=>{
  if(req.url==='/favicon.ico'){res.writeHead(204);res.end();return;} if(req.url==='/'){res.end('<a href="/file?case=keep">download</a>');return;}
  const record={method:req.method,url:req.url,bytes:0,closed:false};transfers.push(record);
  res.writeHead(200,{'Content-Type':'application/octet-stream','Content-Disposition':'attachment; filename="fixture.bin"'});
  let count=0;const timer=setInterval(()=>{if(count++===200){clearInterval(timer);res.end();return;}record.bytes+=4096;res.write(Buffer.alloc(4096));},10);
  res.on('close',()=>{clearInterval(timer);record.closed=true;});
 });await new Promise(r=>server.listen(0,'127.0.0.1',r));
 const base='http://127.0.0.1:'+server.address().port;
 const extension=path.resolve('extensions/chromium');
 let context;
 try{
  context=await chromium.launchPersistentContext(path.resolve('.smoke/browser-stream-profile-'+Date.now()),{executablePath:process.env.STRATAFETCH_TEST_BROWSER || 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',headless:true,ignoreDefaultArgs:['--disable-extensions'],args:['--disable-extensions-except='+extension,'--load-extension='+extension],acceptDownloads:true});
  const worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker',{timeout:15000});
  // Keep browser APIs and extension code real; substitute only the external
  // native companion so this fixture can hold/choose consent deterministically.
  await worker.evaluate(()=>{
   globalThis.prompts=[];globalThis.commits=[];
   nativeRequest=async(payload,beforeCommit)=>{
    if(payload.action!=='capture')throw Error('unexpected native action '+payload.action);
    const choice=await new Promise(resolve=>{globalThis.choose=resolve;prompts.push(payload);});
    if(!choice)return {ok:false,declined:true};
    commits.push(await beforeCommit());return {ok:true,download:{id:'fixture-native-id'}};
   };
  });
  const page=await context.newPage();await page.goto(base);
  async function until(fn){const end=Date.now()+10000;while(!await fn()){if(Date.now()>end)throw Error('fixture timed out');await new Promise(r=>setTimeout(r,20));}}
  for(const accept of [false,true]){
   const name=accept?'accept':'cancel';const before=transfers.length;
   await page.evaluate(url=>{const a=document.createElement('a');a.href=url;document.body.append(a);a.click();},base+'/file?case='+name);
   await until(()=>worker.evaluate(()=>typeof choose==='function'));
   await until(()=>transfers[before]?.closed);
   const bytes=transfers[before].bytes;await new Promise(r=>setTimeout(r,500));
   assert.equal(transfers[before].bytes,bytes,'body must stop while consent is pending');
   assert.equal(transfers.length,before+1,'no browser restart before consent');
   const stopped=await worker.evaluate(()=>chrome.downloads.search({}));
   assert(!stopped.some(d=>d.url.endsWith('case='+name)), 'captured browser entry must be erased before consent');
   await worker.evaluate(accept=>{const resolve=choose;delete globalThis.choose;resolve(accept);},accept);
   if(accept){
    await until(()=>worker.evaluate(()=>commits.length===1));
    await new Promise(r=>setTimeout(r,300));assert.equal(transfers.length,before+1,'accept must not restart browser');
   }else{
    await new Promise(r=>setTimeout(r,500));assert.equal(transfers.length,before+1,'Cancel must not restart browser');
   }
   console.log(JSON.stringify({case:name,stoppedBeforeChoice:true,bytesBeforeCancellation:bytes,browserRestarted:false}));
  }
  const promptsBefore=await worker.evaluate(()=>prompts.length);
  await page.evaluate(url=>{const f=document.createElement('form');f.method='POST';f.action=url;document.body.append(f);f.submit();},base+'/file?case=post');
  await until(()=>transfers.at(-1)?.url.includes('case=post')&&transfers.at(-1).closed);
  assert.equal(await worker.evaluate(()=>prompts.length),promptsBefore,'POST must remain in browser');
  console.log('POST preserved; actual extension/browser cancellation, entry removal and Start/Cancel passed');
 }finally{if(context)await context.close();await new Promise(r=>server.close(r));}
})().catch(e=>{console.error(e);process.exit(1)});
