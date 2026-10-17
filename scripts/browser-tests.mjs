import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {runInNewContext} from 'node:vm';
import {createHash,webcrypto} from 'node:crypto';
const read = (folder,file) => readFileSync(new URL('../extensions/'+folder+'/'+file,import.meta.url),'utf8');
for (const folder of ['chromium', 'firefox']) {
  for (const [settings, item, accepted] of [
    [{captureMinBytes: 1048576}, {totalBytes: 100}, false],
    [{captureMinBytes: 1048576}, {totalBytes: -1}, false],
    [{captureMinBytes: 1048576}, {totalBytes: 2097152}, true],
    [{captureExtensions: ['iso']}, {filename: 'file.zip'}, false],
    [{captureExtensions: ['zip']}, {filename: 'FILE.ZIP'}, true],
    [{captureExtensions: ['tar.gz']}, {filename: 'backup.tar.gz'}, true],
    [{captureMinBytes: -1}, {}, false],
    [{captureExtensions: ['../exe']}, {}, false]
  ]) {
    test(folder+': automatic capture filters '+JSON.stringify([settings,item]),async()=>{
      const b=background(folder,{stored:settings,item});
      await b.hooks.created(b.item);
      assert.equal(b.calls.some(call=>call.payload?.action==='capture'),accepted);
      if (!accepted) assert.equal(b.calls.some(call=>call.op==='pause'),false,'filtered downloads never pause the browser');
    });
  }
  test(folder+': Swedish locale follows app override and preserves filenames',()=>{
    const scope={chrome:{i18n:{getUILanguage:()=> 'sv-SE'}}};
    runInNewContext(read(folder,'locale.js'),scope);
    assert.equal(scope.StrataLocale.t('Download with StrataFetch'),'Hämta med StrataFetch');
    assert.equal(scope.StrataLocale.t('Added: {name}',{name:'My English File.zip'}),'Tillagd: My English File.zip');
    scope.StrataLocale.setLanguage('en');
    assert.equal(scope.StrataLocale.t('Download with StrataFetch'),'Download with StrataFetch');
  });
}
function background(folder, options={}) {
  const calls=[], hooks={}, stored={...(options.stored || {})};
  let enabled=options.enabled ?? true;
  const item={id:7,url:'https://example.org/file.zip',filename:'C:\\Downloads\\file.zip',state:'in_progress',startTime:'2026-10-16T12:00:00Z',danger:'safe',totalBytes:100,mime:'application/zip',...options.item};
  for (const value of (options.items || [])) {
    if (value.filename === undefined) value.filename = 'file.zip';
    if (value.mime === undefined) value.mime = 'application/zip';
  }
  const items = new Map([item, ...(options.items || [])].map(value => [value.id, value]));
  const searches = new Map();
  const api={
    runtime:{onStartup:{addListener(fn){hooks.startup=fn;}},onMessage:{addListener(fn){hooks.message=fn;}},onInstalled:{addListener(fn){hooks.installed=fn;}},async sendNativeMessage(host,payload){
      if(payload.action==='probeReplay'){calls.push({op:'probe',payload});return {ok:true,replayable:Boolean(options.replayable)};}
      calls.push({op:'native',host,payload});
      if(options.native) return options.native(payload);
      return payload.action==='cancel' ? {ok:true} : {ok:true,download:{id:'native-id'}};
    }},
    storage:{local:{async get(keys){return keys===null?{capture:enabled,...stored}:{...keys,...stored,capture:enabled};},async set(value){if ("capture" in value) enabled=value.capture;Object.assign(stored,value);},async remove(key){delete stored[key];}}},
    contextMenus:{async removeAll(){},create(value){calls.push({op:'menu',value});},onClicked:{addListener(fn){hooks.clicked=fn;}}},
    webRequest:{onBeforeRequest:{addListener(fn){hooks.request=fn;}}},
    downloads:{
      onCreated:{addListener(fn){hooks.created=fn;}},
      onChanged:{addListener(fn){hooks.changed=fn;}},
      onDeterminingFilename:{addListener(fn){hooks.filename=fn;}},
      async pause(id){calls.push({op:'pause',id});if(options.pauseWait)await options.pauseWait;if(options.pauseError)throw Error('pause failed');items.get(id).paused=true;},
      async search({id}){calls.push({op:'search',id});const count=(searches.get(id)||0)+1;searches.set(id,count);const changes=typeof options.searchItem==='function'?options.searchItem(id,count):options.searchItem;return items.has(id)?[{...items.get(id),...changes}]:[];},
      async resume(id){calls.push({op:'resume',id});if(options.resumeError)throw Error('resume failed');items.get(id).paused=false;},
      async erase({id}){calls.push({op:'erase',id});items.delete(id);return [id];},
      async download(options){calls.push({op:'browser-download',options});if(options.restartError)throw Error('restart failed');return 99;},
      async removeFile(id){calls.push({op:'removeFile',id});},
      async cancel(id){calls.push({op:'cancel',id});if(options.cancelError)throw Error('cancel failed');if(!options.cancelNoEffect)Object.assign(items.get(id),{state:'interrupted',error:'USER_CANCELED'});},
    },
    action:Object.fromEntries(['setBadgeText','setBadgeBackgroundColor','setTitle'].map(name=>[name,async value=>{calls.push({op:name,...value});if(options.badgeError)throw Error('badge failed');}])),
  };
  runInNewContext(read(folder,'background.js'),{URL,setTimeout,crypto:webcrypto,TextEncoder,[folder==='firefox'?'browser':'chrome']:api});
  return {api,calls,hooks,item,items,stored,enabled:()=>enabled};
}

async function settled(condition) {
 const deadline=Date.now()+3000;
 while(!condition()){if(Date.now()>deadline)throw Error('Timed out waiting for capture');await new Promise(r=>setImmediate(r));}
}
function port(b) {
 let receive,disconnect;const sent=[];
 b.api.runtime.connectNative=()=>({onMessage:{addListener(f){receive=f;}},onDisconnect:{addListener(f){disconnect=f;}},postMessage(p){sent.push(p);},disconnect(){disconnect();}});
 return {sent,reply:value=>receive(value),disconnect:()=>disconnect()};
}
for(const folder of ['chromium','firefox']) {
 for(const accept of [false,true])test(folder+': stop and erase before consent; start/cancel '+accept,async()=>{
  const b=background(folder,{item:{totalBytes:-1,url:'https://example.org/file?token=x'}}),p=port(b);
  const work=b.hooks.created(b.item);await settled(()=>p.sent.length);
  assert.equal(b.item.state,'interrupted');assert.equal(b.items.has(7),false);
  assert.equal(b.calls.some(c=>c.op==='probe'),false);
  assert.equal(p.sent.some(p=>p.action==='commitCapture'),false);
  if(accept){p.reply({ok:true,confirmation:'accepted'});await settled(()=>p.sent.some(p=>p.action==='commitCapture'));p.reply({ok:true,download:{id:'native-id'}});}
  else p.reply({ok:false,declined:true});
  await work;
  assert.equal(b.calls.some(c=>['resume','browser-download'].includes(c.op)),false);
  assert.equal('capture_pending_7' in b.stored,false);
 });
 for(const failed of ['forbidden','disconnected','declined'])test(folder+': '+failed+' never recreates browser transfer',async()=>{
  const b=background(folder,{native:async()=>{if(failed==='forbidden')throw Error('Access to the specified native messaging host is forbidden.');return failed==='declined'?{ok:false,declined:true}:{ok:false,error:'Native host disconnected'};}});
  await b.hooks.created(b.item);
  assert.equal(b.items.has(7),false);assert.equal(b.calls.some(c=>['resume','browser-download'].includes(c.op)),false);
  if(failed!=='declined')assert.match(b.stored.lastHandoffError.message,/forbidden|disconnected/);
 });
 test(folder+': cancellation failure prevents native consent and preserves cleanup record',async()=>{
  const b=background(folder,{cancelNoEffect:true});await b.hooks.created(b.item);
  assert.equal(b.calls.some(c=>c.op==='native'),false);assert('capture_pending_7' in b.stored);
 });
 test(folder+': completed race removes only captured browser file before native request',async()=>{
  const b=background(folder);b.api.downloads.cancel=async id=>{b.item.state='complete';};
  await b.hooks.created(b.item);
  const removal=b.calls.findIndex(c=>c.op==='removeFile');assert(removal>=0);
  assert(removal<b.calls.findIndex(c=>c.op==='erase'));assert(removal<b.calls.findIndex(c=>c.op==='native'));
 });
 for(const item of [{incognito:true},{byExtensionId:'another-extension'},{danger:'dangerous'},{url:'blob:x'},{mime:'text/html'},{filename:'plugin.xpi'}])test(folder+': unsupported download untouched '+JSON.stringify(item),async()=>{
  const b=background(folder,{item});await b.hooks.created(b.item);assert.equal(b.calls.some(c=>['cancel','native','erase','removeFile'].includes(c.op)),false);
 });
 test(folder+': observed POST stays untouched',async()=>{
  const b=background(folder);b.hooks.request({requestId:'post',url:b.item.url,method:'POST'});await b.hooks.created(b.item);
  assert.equal(b.calls.some(c=>['cancel','native','erase'].includes(c.op)),false);
 });
 test(folder+': duplicate browser events create one prompt',async()=>{
  const b=background(folder),p=port(b);const first=b.hooks.created(b.item);await b.hooks.created(b.item);
  await settled(()=>p.sent.length);p.reply({ok:false,declined:true});await first;assert.equal(p.sent.length,1);
 });
 test(folder+': simultaneous transfers stop and erase while native prompts remain serialized',async()=>{
  const second={id:8,url:'https://example.org/other.zip',state:'in_progress',startTime:'second'};
  const finishes=[];const b=background(folder,{items:[second],native:()=>new Promise(r=>finishes.push(r))});
  const a=b.hooks.created(b.item),c=b.hooks.created(second);
  await settled(()=>b.items.size===0&&finishes.length===1);finishes[0]({ok:false,declined:true});
  await settled(()=>finishes.length===2);finishes[1]({ok:false,declined:true});await Promise.all([a,c]);
  assert.equal(b.calls.some(c=>c.op==='browser-download'),false);
 });
 for(const state of ['in_progress','interrupted','complete'])test(folder+': startup cleanup never resumes/restarts '+state,async()=>{
  const b=background(folder,{item:{state,paused:true,error:'USER_CANCELED'},stored:{capture_pending_7:{startTime:'2026-10-16T12:00:00Z',exclusive:{url:'https://example.org/file.zip'}}}});
  await b.hooks.startup();assert.equal(b.calls.some(c=>['resume','browser-download','removeFile'].includes(c.op)),false);
  if(state==='complete')assert(b.items.has(7));else assert(!b.items.has(7));
 });
 test(folder+': current permissions include method observation, never cookies or page scripts',()=>{
  const m=JSON.parse(read(folder,'manifest.json'));assert(m.permissions.includes('webRequest'));assert(!m.permissions.includes('cookies'));assert.equal(m.content_scripts,undefined);
 });
}

for(const folder of ["chromium","firefox"]) {
  test(folder+': popup retains the last failure even when a new connection ping succeeds',async()=>{
    const elements=Object.fromEntries(['status','send','current','url','capture','excluded','save-excluded','last-issue'].map(id=>[id,{}]));
    const api={runtime:{async sendMessage(){return {ok:true,active:0};}},storage:{local:{async get(){return {capture:true,lastHandoffError:{message:'Opening browser bridge: Host exited'}};}}}};
    runInNewContext(read(folder,'popup.js'),{URL,[folder==='firefox'?'browser':'chrome']:api,document:{getElementById:id=>elements[id]}});
    await new Promise(resolve=>setImmediate(resolve));
    assert.match(elements.status.textContent,/Connected/);
    assert.match(elements['last-issue'].textContent,/Last handoff issue: Opening browser bridge: Host exited/);
  });
  test(folder+': installation preserves disabled capture',async()=>{
    const b=background(folder,{enabled:false});await b.hooks.installed();assert.equal(b.enabled(),false);
    assert.deepEqual(Array.from(b.calls[0].value.contexts),['link']);
  });
  test(folder+': late startup ping does not overwrite a current-tab error',async()=>{
    const elements=Object.fromEntries(['status','send','current','url','capture','excluded','save-excluded'].map(id=>[id,{value:'previous'}]));let ping;
    const api={runtime:{sendMessage(){return new Promise(resolve=>{ping=resolve;});}},
      storage:{local:{async get(){return {capture:false};},async set(){}}},tabs:{async query(){throw Error();}}};
    runInNewContext(read(folder,'popup.js'),{URL,[folder==='firefox'?'browser':'chrome']:api,document:{getElementById:id=>elements[id]}});
    await elements.current.onclick();const error=elements.status.textContent;ping({ok:true,active:0});await new Promise(resolve=>setImmediate(resolve));await new Promise(resolve=>setImmediate(resolve));
    assert.equal(elements.status.textContent,error);
  });
  test(folder+': popup relay keeps native work alive if its response view closes',async()=>{
    const b=background(folder);let finish;
    b.api.runtime.sendNativeMessage=()=>new Promise(resolve=>{finish=resolve;});
    const kept=b.hooks.message({action:'capture',url:'https://example.org/file.zip'},{},()=>{throw Error('popup closed');});
    assert.equal(kept,true);
    assert.equal(b.hooks.message({action:'unknown'},{},()=>{}),false);
    while(!finish)await new Promise(resolve=>setImmediate(resolve));
    finish({ok:true,download:{id:'native-id'}});
    await new Promise(resolve=>setImmediate(resolve));await new Promise(resolve=>setImmediate(resolve));
  });
}

for(const folder of ['chromium','firefox']) {
 for(const change of [{url:'https://other.example/file.zip'},{danger:'dangerous'},{incognito:true},{filename:'unsafe.xpi'},{byExtensionId:'other'}])test(folder+': recheck changed metadata before transmission '+JSON.stringify(change),async()=>{
  const b=background(folder,{searchItem:change});await b.hooks.created(b.item);assert.equal(b.calls.some(c=>['native','cancel','erase'].includes(c.op)),false);
 });
 for(const url of ['https://example.org/a.zip','https://cdn.example.org/a.zip'])test(folder+': domain exclusions '+url,async()=>{
  const b=background(folder,{item:{url},stored:{excludedDomains:['example.org']}});await b.hooks.created(b.item);assert.equal(b.calls.some(c=>['native','cancel','erase'].includes(c.op)),false);
 });
 test(folder+': lost native reply leaves browser stopped and never creates another transfer',async()=>{
  const b=background(folder),p=port(b);const work=b.hooks.created(b.item);await settled(()=>p.sent.length);p.reply({ok:true,confirmation:'accepted'});await settled(()=>p.sent.length===2);p.disconnect();await work;
  assert.equal(b.items.has(7),false);assert.equal(b.calls.some(c=>['resume','browser-download'].includes(c.op)),false);
 });
 test(folder+': recovery never cancels an acknowledged native job or another browser identity',async()=>{
  const b=background(folder,{stored:{capture_pending_7:{startTime:'unrelated',nativeId:'accepted'}}});await b.hooks.startup();assert(b.items.has(7));assert.equal(b.calls.some(c=>['native','cancel','erase'].includes(c.op)),false);
 });
}
