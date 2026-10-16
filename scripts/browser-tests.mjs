import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {runInNewContext} from 'node:vm';
import {createHash} from 'node:crypto';
const read = (folder,file) => readFileSync(new URL('../extensions/'+folder+'/'+file,import.meta.url),'utf8');
function background(folder, options={}) {
  const calls=[], hooks={}, stored={...(options.stored || {})};
  let enabled=options.enabled ?? true;
  const item={id:7,url:'https://example.org/file.zip',filename:'C:\\Downloads\\file.zip',state:'in_progress',startTime:'2026-10-16T12:00:00Z',danger:'safe',totalBytes:100,mime:'application/zip',...options.item};
  const items = new Map([item, ...(options.items || [])].map(value => [value.id, value]));
  const api={
    runtime:{onStartup:{addListener(fn){hooks.startup=fn;}},onMessage:{addListener(fn){hooks.message=fn;}},onInstalled:{addListener(fn){hooks.installed=fn;}},async sendNativeMessage(host,payload){
      calls.push({op:'native',host,payload});
      if(options.native) return options.native(payload);
      return payload.action==='cancel' ? {ok:true} : {ok:true,download:{id:'native-id'}};
    }},
    storage:{local:{async get(keys){return keys===null?{capture:enabled,...stored}:{...keys,...stored,capture:enabled};},async set(value){if ("capture" in value) enabled=value.capture;Object.assign(stored,value);},async remove(key){delete stored[key];}}},
    contextMenus:{async removeAll(){},create(value){calls.push({op:'menu',value});},onClicked:{addListener(fn){hooks.clicked=fn;}}},
    downloads:{
      onCreated:{addListener(fn){hooks.created=fn;}},
      async pause(id){calls.push({op:'pause',id});if(options.pauseError)throw Error('pause failed');items.get(id).paused=true;},
      async search({id}){calls.push({op:'search',id});return items.has(id)?[{...items.get(id),...options.searchItem}]:[];},
      async resume(id){calls.push({op:'resume',id});if(options.resumeError)throw Error('resume failed');items.get(id).paused=false;},
      async cancel(id){calls.push({op:'cancel',id});if(options.cancelError)throw Error('cancel failed');items.get(id).state='interrupted';},
    },
    action:Object.fromEntries(['setBadgeText','setBadgeBackgroundColor','setTitle'].map(name=>[name,async value=>{calls.push({op:name,...value});if(options.badgeError)throw Error('badge failed');}])),
  };
  runInNewContext(read(folder,'background.js'),{URL,[folder==='firefox'?'browser':'chrome']:api});
  return {api,calls,hooks,item,items,stored,enabled:()=>enabled};
}
for(const folder of ['chromium','firefox']) {
  test(folder+': native launch failures retain the reason without signed URLs',async()=>{
    const b=background(folder,{native:async()=>{throw Error('Native host exited https://example.org/private?token=secret');}});
    await b.hooks.created(b.item);
    assert.match(b.stored.lastHandoffError.message,/Opening browser bridge: Native host exited/);
    assert.equal(b.stored.lastHandoffError.message.includes('secret'),false);
    assert.equal(b.calls.some(c=>c.op==='resume'),true);
    assert.equal(b.calls.some(c=>c.op==='cancel'),false);
  });
  test(folder+': successful capture clears the previous handoff issue',async()=>{
    const b=background(folder,{stored:{lastHandoffError:{message:'old failure'}}});
    await b.hooks.created(b.item);
    assert.equal('lastHandoffError' in b.stored,false);
  });
  test(folder+': popup retains the last failure even when a new connection ping succeeds',async()=>{
    const elements=Object.fromEntries(['status','send','current','url','capture','excluded','save-excluded','last-issue'].map(id=>[id,{}]));
    const api={runtime:{async sendMessage(){return {ok:true,active:0};}},storage:{local:{async get(){return {capture:true,lastHandoffError:{message:'Opening browser bridge: Host exited'}};}}}};
    runInNewContext(read(folder,'popup.js'),{URL,[folder==='firefox'?'browser':'chrome']:api,document:{getElementById:id=>elements[id]}});
    await new Promise(resolve=>setImmediate(resolve));
    assert.match(elements.status.textContent,/Connected/);
    assert.match(elements['last-issue'].textContent,/Last handoff issue: Opening browser bridge: Host exited/);
  });
  test(folder+': only native/download/storage permissions, no cookies or broad page access',()=>{
    const m=JSON.parse(read(folder,'manifest.json'));
    assert.deepEqual(m.permissions.slice().sort(),['activeTab','contextMenus','downloads','nativeMessaging','storage']);
    assert.equal(m.host_permissions,undefined);assert.equal(m.content_scripts,undefined);
  });
  test(folder+': installation preserves disabled capture',async()=>{
    const b=background(folder,{enabled:false});await b.hooks.installed();assert.equal(b.enabled(),false);
    assert.deepEqual(Array.from(b.calls[0].value.contexts),['link']);
  });
  for(const result of ['success','declined','rejected','unavailable']) {
    test(folder+': context menu '+result,async()=>{
      const b=background(folder,{native:async()=>{if(result==='unavailable')throw Error();return result==='success'?{ok:true}:result==='declined'?{ok:false,declined:true}:{ok:false,error:'Unsupported URL'};}});
      await b.hooks.clicked({menuItemId:'stratafetch',linkUrl:'https://example.org/a.zip'});
      assert.equal(b.calls[0].payload.action,'capture');
      assert.deepEqual(Object.keys(b.calls[0].payload).sort(),['action','url']);
      const count=b.calls.length;await b.hooks.clicked({menuItemId:'other'});assert.equal(b.calls.length,count);
    });
  }
  test(folder+': browser is cancelled only after native acknowledgement',async()=>{
    const b=background(folder);await b.hooks.created(b.item);
    assert.equal(b.calls.findIndex(c=>c.op==='pause'),0);
    const native=b.calls.findIndex(c=>c.op==='native');
    assert(native>0);assert(b.calls.findIndex(c=>c.op==='cancel')>native);
    assert.equal(b.calls.some(c=>c.op==='resume'),false);
    const payload=b.calls[native].payload;
    assert.deepEqual(Object.keys(payload).sort(),['action','mime','name','size','url']);
    assert.equal(payload.name,'file.zip');assert.equal(payload.size,100);
  });
  for(const failure of ['decline','reject','unavailable','missing-id','pause','cancel','changed','badge']) {
    test(folder+': handoff '+failure+' preserves the browser download',async()=>{
      const b=background(folder,{
        pauseError:failure==='pause',cancelError:failure==='cancel',badgeError:failure==='badge',
        searchItem:failure==='changed'?{danger:'dangerous'}:undefined,
        native:async payload=>{
          if(payload.action==='cancel')return {ok:true};
          if(failure==='unavailable')throw Error('host missing');
          if(failure==='decline')return {ok:false,declined:true};
          if(failure==='reject')return {ok:false,error:'server rejected request'};
          return {ok:true,download:failure==='missing-id'?{}:{id:'native-id'}};
        }
      });
      await b.hooks.created(b.item);
      if(failure==='badge')assert.equal(b.calls.some(c=>c.op==='cancel'),true);
      else if(failure==='pause')assert.equal(b.calls.some(c=>c.op==='native'),false);
      else {
        assert.equal(b.calls.some(c=>c.op==='resume'),true);
        if(failure==='cancel')assert.equal(b.calls.some(c=>c.payload?.action==='cancel'),true);
      }
    });
  }
  for(const item of [{incognito:true},{byExtensionId:'another'},{state:'complete'},{paused:true},{url:'blob:secret'},{url:'data:text/plain,hi'},{url:'https://user:secret@example.org/file.zip'},{url:'http://['},{mime:'application/x-chrome-extension'},{danger:'dangerous'},{mime:'text/html'},{url:'https://example.org/extension.xpi'},{url:'https://example.org/extension.crx'}]) {
    test(folder+': does not capture '+JSON.stringify(item),async()=>{
      const b=background(folder,{item});await b.hooks.created(b.item);assert.equal(b.calls.length,0);
    });
  }
  test(folder+': disabled capture does not pause downloads',async()=>{
    const b=background(folder,{enabled:false});await b.hooks.created(b.item);assert.equal(b.calls.length,0);
  });
  test(folder+': duplicate events cannot create duplicate native transfers',async()=>{
    let reply;const b=background(folder,{native:()=>new Promise(resolve=>{reply=resolve;})});
    const pending=b.hooks.created(b.item);
    while(!reply)await Promise.resolve();
    await b.hooks.created(b.item);
    assert.equal(b.calls.filter(c=>c.op==='native').length,1);
    assert.equal(b.calls.some(c=>c.op==='cancel'),false);
    reply({ok:true,download:{id:'native-id'}});await pending;
    assert.equal(b.calls.filter(c=>c.op==='cancel').length,1);
  });
  test(folder+': native port remains connected while awaiting confirmation',async()=>{
    const b=background(folder);let message,disconnect;let closed=false;
    b.api.runtime.connectNative=()=>({
      onMessage:{addListener(fn){message=fn;}},onDisconnect:{addListener(fn){disconnect=fn;}},
      postMessage(){},disconnect(){closed=true;disconnect();}
    });
    const pending=b.hooks.created(b.item);
    while(!message)await Promise.resolve();
    assert.equal(closed,false);assert.equal(b.calls.some(c=>c.op==='cancel'),false);
    message({ok:false,declined:true});await pending;
    assert.equal(closed,true);assert.equal(b.calls.some(c=>c.op==='resume'),true);
  });
  test(folder+': a failed native send closes its port and resumes the browser',async()=>{
    const b=background(folder);let closed=false;
    b.api.runtime.connectNative=()=>({onMessage:{addListener(){}},onDisconnect:{addListener(){}},postMessage(){throw Error('send failed');},disconnect(){closed=true;}});
    await b.hooks.created(b.item);
    assert(closed);assert.equal(b.calls.some(c=>c.op==='resume'),true);
    assert.equal(b.calls.some(c=>c.op==='cancel'),false);
  });
  for(const result of ['empty','restricted','error']) {
    test(folder+': current page '+result+' has an actionable error',async()=>{
      const elements=Object.fromEntries(['status','send','current','url','capture','excluded','save-excluded'].map(id=>[id,{value:'previous URL'}]));
      const api={runtime:{async sendMessage(){return {ok:true,active:0};}},storage:{local:{async get(){return {capture:true};},async set(){}}},
        tabs:{async query(){if(result==='error')throw Error();return result==='empty'?[]:[{id:1}];}}};
      runInNewContext(read(folder,'popup.js'),{URL,[folder==='firefox'?'browser':'chrome']:api,document:{getElementById:id=>elements[id]}});
      await Promise.resolve();await elements.current.onclick();
      assert.equal(elements.url.value,'');assert.match(elements.status.textContent,/cannot read.*paste/i);
    });
  }
  test(folder+': late startup ping does not overwrite a current-tab error',async()=>{
    const elements=Object.fromEntries(['status','send','current','url','capture','excluded','save-excluded'].map(id=>[id,{value:'previous'}]));let ping;
    const api={runtime:{sendMessage(){return new Promise(resolve=>{ping=resolve;});}},
      storage:{local:{async get(){return {capture:false};},async set(){}}},tabs:{async query(){throw Error();}}};
    runInNewContext(read(folder,'popup.js'),{URL,[folder==='firefox'?'browser':'chrome']:api,document:{getElementById:id=>elements[id]}});
    await elements.current.onclick();const error=elements.status.textContent;ping({ok:true,active:0});await Promise.resolve();await Promise.resolve();
    assert.equal(elements.status.textContent,error);
  });
}
test('local Chromium ID derives from the bundled public key and is independent of install path',()=>{
  const m=JSON.parse(read('chromium','manifest.json'));
  const hash=createHash('sha256').update(Buffer.from(m.key,'base64')).digest().subarray(0,16);
  const id=[...hash].flatMap(b=>[b>>4,b&15]).map(n=>String.fromCharCode(97+n)).join('');
  const ids=JSON.parse(readFileSync(new URL('../extensions/local-ids.json',import.meta.url)));
  assert.equal(id,ids.chromium);
  assert.equal(JSON.parse(read('firefox','manifest.json')).browser_specific_settings.gecko.id,ids.firefox);
});

for (const folder of ['chromium','firefox']) {
  test(folder+': popup relay keeps native work alive if its response view closes',async()=>{
    const b=background(folder);let finish;
    b.api.runtime.sendNativeMessage=()=>new Promise(resolve=>{finish=resolve;});
    const kept=b.hooks.message({action:'capture',url:'https://example.org/file.zip'},{},()=>{throw Error('popup closed');});
    assert.equal(kept,true);
    assert.equal(b.hooks.message({action:'unknown'},{},()=>{}),false);
    while(!finish)await Promise.resolve();
    finish({ok:true,download:{id:'native-id'}});
    await Promise.resolve();await Promise.resolve();
  });
}


for (const folder of ['chromium','firefox']) {
  for(const url of ['https://example.org/file.zip','https://cdn.example.org/file.zip']) {
    test(folder+': excluded domain remains in browser '+url,async()=>{
      const b=background(folder,{item:{url},stored:{excludedDomains:['example.org']}});
      await b.hooks.created(b.item);assert.equal(b.calls.length,0);
    });
  }
  test(folder+': unrelated domains are still captured',async()=>{
    const b=background(folder,{stored:{excludedDomains:['other.example.org']}});
    await b.hooks.created(b.item);assert(b.calls.some(call=>call.op==='cancel'));
  });
  test(folder+': exclusions added while waiting prevent a queued native prompt',async()=>{
    const next={id:8,url:'https://second.example.org/file.zip',state:'in_progress',startTime:'second'};
    let finish;const b=background(folder,{items:[next],native:()=>new Promise(resolve=>{finish=resolve;})});
    const first=b.hooks.created(b.item),second=b.hooks.created(next);
    while(!finish)await Promise.resolve();
    await b.api.storage.local.set({excludedDomains:['second.example.org']});
    finish({ok:false,declined:true});await Promise.all([first,second]);
    assert.equal(b.calls.filter(call=>call.op==='native').length,1);
    assert(b.calls.some(call=>call.op==='resume'&&call.id===8));
  });
  for(const invalid of [false,true]) {
    test(folder+': popup validates domain exclusions '+invalid,async()=>{
      const elements=Object.fromEntries(['status','send','current','url','capture','excluded','save-excluded'].map(id=>[id,{}]));
      let saved;
      const api={runtime:{async sendMessage(){return {ok:true};}},storage:{local:{async get(){return {capture:true};},async set(value){saved=value;}}}};
      runInNewContext(read(folder,'popup.js'),{URL,[folder==='firefox'?'browser':'chrome']:api,document:{getElementById:id=>elements[id]}});
      await new Promise(resolve=>setImmediate(resolve));
      elements.excluded.value=invalid?'example.org/path':'EXAMPLE.ORG\n*.cdn.example.org\nexample.org';
      await elements['save-excluded'].onclick();
      if(invalid) {assert.equal(saved,undefined);assert.match(elements.status.textContent,/without URLs/);}
      else {assert.deepEqual(Array.from(saved.excludedDomains),['example.org','cdn.example.org']);assert.match(elements.status.textContent,/Exclusions saved/);}
    });
  }
  test(folder+': simultaneous downloads pause immediately and prompt one at a time', async()=>{
    const next={id:8,url:'https://example.org/second.zip',filename:'second.zip',state:'in_progress',startTime:'second'};
    const finishes=[];
    const b=background(folder,{items:[next],native:()=>new Promise(resolve=>finishes.push(resolve))});
    const first=b.hooks.created(b.item), second=b.hooks.created(next);
    while(!finishes.length)await Promise.resolve();
    assert.equal(b.calls.filter(call=>call.op==='pause').length,2);
    assert.equal(finishes.length,1);
    finishes[0]({ok:false,declined:true});
    while(finishes.length<2)await Promise.resolve();
    finishes[1]({ok:false,declined:true});await Promise.all([first,second]);
    assert.equal(b.calls.filter(call=>call.op==='resume').length,2);
  });
  test(folder+': a cancelled queued download never opens a native confirmation', async()=>{
    const next={id:8,url:'https://example.org/second.zip',state:'in_progress',startTime:'second'};
    let finish;const b=background(folder,{items:[next],native:()=>new Promise(resolve=>{finish=resolve;})});
    const first=b.hooks.created(b.item),second=b.hooks.created(next);
    while(!finish)await Promise.resolve();
    next.state='interrupted';finish({ok:false,declined:true});await Promise.all([first,second]);
    assert.equal(b.calls.filter(call=>call.op==='native').length,1);
    assert.equal(b.calls.some(call=>call.op==='resume'&&call.id===8),false);
  });
  for(const searchItem of [{incognito:true},{url:'blob:private',finalUrl:'blob:private'},{mime:'text/html'},{filename:'extension.xpi'},{startTime:'different'}]) {
    test(folder+': changed download is rechecked before URL transmission '+JSON.stringify(searchItem),async()=>{
      const b=background(folder,{searchItem});await b.hooks.created(b.item);
      assert.equal(b.calls.some(call=>call.op==='native'),false);
      assert.equal(b.calls.some(call=>call.op==='cancel'),false);
    });
  }
  test(folder+': failed resume retains recovery data and retries successfully', async()=>{
    const options={resumeError:true,native:async()=>({ok:false,declined:true})};
    const b=background(folder,options);await b.hooks.created(b.item);
    assert.equal(b.item.paused,true);assert('capture_pending_7' in b.stored);
    await b.hooks.startup();assert('capture_pending_7' in b.stored);
    options.resumeError=false;await b.hooks.startup();
    assert.equal(b.item.paused,false);assert.equal('capture_pending_7' in b.stored,false);
  });
  test(folder+': recovery rolls back an acknowledged native copy before browser resume',async()=>{
    const b=background(folder,{item:{paused:true},stored:{capture_pending_7:{startTime:'2026-10-16T12:00:00Z',nativeId:'native-id'}}});
    await b.hooks.startup();
    const cancel=b.calls.findIndex(call=>call.payload?.action==='cancel');
    assert(cancel>=0);assert(b.calls.findIndex(call=>call.op==='resume')>cancel);
  });
  for(const reply of [{ok:false,error:'Native host unavailable'},undefined,{ok:true,running:false}]) {
    test(folder+': popup accurately reports bridge readiness '+JSON.stringify(reply),async()=>{
      const elements=Object.fromEntries(['status','send','current','url','capture','excluded','save-excluded'].map(id=>[id,{}]));
      const api={runtime:{async sendMessage(){return reply;}},storage:{local:{async get(){return {capture:true};}}}};
      runInNewContext(read(folder,'popup.js'),{URL,[folder==='firefox'?'browser':'chrome']:api,document:{getElementById:id=>elements[id]}});
      await new Promise(resolve=>setImmediate(resolve));
      assert.match(elements.status.textContent,reply?.ok?/Ready.*opens/:/unavailable|did not respond/i);
    });
  }
  for (const state of ['owned','different-download','already-resumed']) {
    test(folder+': restart recovery '+state+' affects only a paused owned download', async()=>{
      const b=background(folder,{
        item:{paused:state!=='already-resumed'},
        stored:{capture_pending_7:{startTime:state==='different-download'?'old-start':'2026-10-16T12:00:00Z'}}
      });
      await b.hooks.startup();
      assert.equal(b.calls.some(call=>call.op==='resume'),state==='owned');
      assert.equal('capture_pending_7' in b.stored,false);
      assert.equal(b.calls.some(call=>call.op==='native'),false);
    });
  }
}
