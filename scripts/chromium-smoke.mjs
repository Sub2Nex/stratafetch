// Real Chromium extension -> native host -> IPC -> HTTP integration.
// Set CHROMIUM_BIN to a trusted local Chromium executable; no installed profiles are modified.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdtemp, mkdir, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
const exec = promisify(execFile);
const root = fileURLToPath(new URL('../', import.meta.url));
async function stop(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  await new Promise(resolve => {
    const timer = setTimeout(() => child.kill('SIGKILL'), 3000);
    child.once('exit', () => { clearTimeout(timer); resolve(); });
    child.kill('SIGTERM');
  });
}
async function connect(url) {
  const socket = new WebSocket(url);
  await new Promise((resolve, reject) => { socket.addEventListener('open', resolve, { once: true }); socket.addEventListener('error', reject, { once: true }); });
  const pending = new Map(); let next = 0;
  socket.addEventListener('message', event => {
    const message = JSON.parse(event.data);
    if (pending.has(message.id)) { const request = pending.get(message.id); pending.delete(message.id); clearTimeout(request.timer); message.error ? request.reject(new Error(JSON.stringify(message.error))) : request.resolve(message.result); }
  });
  return {
    socket,
    command(method, params = {}, sessionId) {
      return new Promise((resolve, reject) => {
        const id = ++next;
        const timer = setTimeout(() => { pending.delete(id); reject(new Error(`${method} timed out`)); }, 15000);
        pending.set(id, { resolve, reject, timer });
        socket.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }));
      });
    },
  };
}
test('real Chromium hands a URL to the native host and completes the exact download', { skip: !process.env.CHROMIUM_BIN || process.platform !== 'linux', timeout: 60000 }, async () => {
  const folder = await mkdtemp(join(tmpdir(), 'stratafetch-chromium-'));
  const config = join(folder, 'config'); const profile = join(folder, 'browser-profile');
  await mkdir(profile, { recursive: true });
  const state = join(folder, 'state'); const downloads = join(folder, 'downloads');
  const env = { ...process.env, XDG_CONFIG_HOME: config, XDG_CACHE_HOME: join(folder, 'cache'), STRATAFETCH_STATE_DIR: state, STRATAFETCH_DOWNLOAD_DIR: downloads };
  const binaries = resolve(process.env.STRATAFETCH_BIN_DIR || join(root, 'target/debug'));
  const bytes = Buffer.from(Array.from({ length: 512 * 1024 }, (_, i) => i % 251));
  const http = createServer((req, res) => {
    let first = 0, last = bytes.length - 1;
    const range = req.headers.range && /^bytes=(\d+)-(\d+)$/.exec(req.headers.range);
    if (range) { first = Number(range[1]); last = Math.min(Number(range[2]), last); res.setHeader('Content-Range', `bytes ${first}-${last}/${bytes.length}`); }
    res.writeHead(range ? 206 : 200, { ETag: '"chromium-fixture"', 'Content-Length': last - first + 1 }); res.end(bytes.subarray(first, last + 1));
  });
  await new Promise(resolve => http.listen(0, '127.0.0.1', resolve));
  let chromium, service, debuggerClient; let browserErrors = '', serviceErrors = '';
  try {
    const extension = join(root, 'extensions/chromium');
    chromium = spawn(process.env.CHROMIUM_BIN, [...(process.env.DISPLAY ? [] : ['--headless=new']), '--disable-gpu', '--enable-extensions', '--enable-logging=stderr', '--disable-background-networking', '--disable-component-update', '--disable-sync', '--no-first-run', '--no-default-browser-check', '--remote-debugging-port=0', `--user-data-dir=${profile}`, `--disable-extensions-except=${extension}`, `--load-extension=${extension}`, 'about:blank'], { env, stdio: ['ignore', 'ignore', 'pipe'] });
    chromium.stderr.on('data', data => { browserErrors += data; });
    let endpoint;
    for (let attempt = 0; attempt < 150; attempt++) {
      try { const [port, path] = (await readFile(join(profile, 'DevToolsActivePort'), 'utf8')).trim().split('\n'); endpoint = `ws://127.0.0.1:${port}${path}`; break; }
      catch { assert.ok(!browserErrors.includes('Loading of unpacked extensions is disabled by the administrator'), 'Chromium administrator policy prevents this integration check'); assert.equal(chromium.exitCode, null, browserErrors); await delay(100); }
    }
    assert.ok(endpoint, browserErrors);
    debuggerClient = await connect(endpoint);
    let worker;
    for (let attempt = 0; attempt < 100; attempt++) {
      const targets = await debuggerClient.command('Target.getTargets', { filter: [{ type: 'service_worker' }] });
      worker = targets.targetInfos.find(target => target.type === 'service_worker' && target.url.startsWith('chrome-extension://'));
      if (worker) break; assert.ok(!browserErrors.includes('Loading of unpacked extensions is disabled by the administrator'), 'Chromium administrator policy prevents this integration check'); await delay(100);
    }
    assert.ok(worker, `Extension worker missing: ${browserErrors}`);
    const id = new URL(worker.url).hostname;
    await exec(join(binaries, 'stratafetch-browser-host'), ['--register', '--chromium-id', id], { env });
    service = spawn(join(binaries, 'stratafetch'), ['serve'], { env, stdio: ['ignore', 'ignore', 'pipe'] });
    service.stderr.on('data', data => { serviceErrors += data; });
    const list = async () => JSON.parse((await exec(join(binaries, 'stratafetch'), ['--require-service', '--json', 'list'], { env, timeout: 10000 })).stdout);
    let ready = false;
    for (let attempt = 0; attempt < 100; attempt++) { try { await list(); ready = true; break; } catch { assert.equal(service.exitCode, null, serviceErrors); await delay(20); } }
    assert.ok(ready, serviceErrors);
    const { sessionId } = await debuggerClient.command('Target.attachToTarget', { targetId: worker.targetId, flatten: true });
    const native = async payload => {
      const result = await debuggerClient.command('Runtime.evaluate', { expression: `chrome.runtime.sendNativeMessage('org.stratafetch.host', ${JSON.stringify(payload)})`, awaitPromise: true, returnByValue: true }, sessionId);
      assert.equal(result.exceptionDetails, undefined, JSON.stringify(result.exceptionDetails)); return result.result.value;
    };
    assert.equal((await native({ action: 'ping' })).ok, true);
    const added = await native({ action: 'add', url: `http://127.0.0.1:${http.address().port}/r%C3%A4ksm%C3%B6rg%C3%A5s%20final.zip` });
    assert.equal(added.ok, true, added.error);
    assert.equal(added.download.filename, 'räksmörgås final.zip');
    let job;
    for (let attempt = 0; attempt < 200; attempt++) { job = (await list()).jobs.find(job => job.id === added.download.id); if (job.status === 'Completed') break; assert.notEqual(job.status, 'Failed', job.error); await delay(20); }
    assert.equal(job.status, 'Completed'); assert.deepEqual(await readFile(job.destination), bytes);
    assert.equal(job.sha256, createHash('sha256').update(bytes).digest('hex'));
    await exec(join(binaries, 'stratafetch'), ['--require-service', 'verify', job.id], { env });
    await exec(join(binaries, 'stratafetch-browser-host'), ['--unregister'], { env });
  } finally {
    debuggerClient?.socket.close(); await stop(chromium); await stop(service);
    await new Promise(resolve => http.close(resolve)); await rm(folder, { recursive: true, force: true });
  }
});
