// Run after cargo build --workspace --locked. Uses isolated state and loopback HTTP only.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { setTimeout as delay } from 'node:timers/promises';
const exec = promisify(execFile);
const root = fileURLToPath(new URL('../', import.meta.url));
const binaries = resolve(process.env.STRATAFETCH_BIN_DIR || join(root, 'target/debug'));
async function stop(child) {
  if (child.exitCode !== null || child.signalCode !== null) return;
  await new Promise(resolve => {
    const timer = setTimeout(() => child.kill('SIGKILL'), 3000);
    child.once('exit', () => { clearTimeout(timer); resolve(); });
    child.kill('SIGINT');
  });
}
async function native(env, origin, payload) {
  const body = Buffer.from(JSON.stringify(payload));
  const prefix = Buffer.alloc(4); prefix.writeUInt32LE(body.length);
  const child = spawn(join(binaries, 'stratafetch-browser-host'), [origin], { env });
  const stdout = [], stderr = [];
  const result = new Promise((resolve, reject) => {
    const timer = setTimeout(() => { child.kill('SIGKILL'); reject(new Error('Native host timeout')); }, 15000);
    child.on('error', error => { clearTimeout(timer); reject(error); });
    child.stdout.on('data', data => stdout.push(data)); child.stderr.on('data', data => stderr.push(data));
    child.stdin.on('error', error => { if (error.code !== 'EPIPE') reject(error); });
    child.on('exit', (code, signal) => { clearTimeout(timer); resolve({ code, signal, stdout: Buffer.concat(stdout), stderr: Buffer.concat(stderr).toString() }); });
  });
  child.stdin.end(Buffer.concat([prefix, body]));
  return result;
}
test('Unix CLI service and real native host complete an isolated ranged download', { skip: process.platform === 'win32', timeout: 30000 }, async () => {
  const folder = await mkdtemp(join(tmpdir(), 'stratafetch-unix-'));
  // Deliberately exceed macOS's Unix socket path limit in the state path.
  const state = join(folder, 'long-state-directory-'.repeat(7), 'state');
  await mkdir(state, { recursive: true });
  const origin = `chrome-extension://${'a'.repeat(32)}/`;
  await writeFile(join(state, 'browser-origins.json'), JSON.stringify([origin]));
  const env = { ...process.env, STRATAFETCH_STATE_DIR: state, STRATAFETCH_DOWNLOAD_DIR: join(folder, 'downloads') };
  const bytes = Buffer.from(Array.from({ length: 1024 * 1024 }, (_, i) => i % 251));
  const server = createServer((req, res) => {
    let first = 0, last = bytes.length - 1;
    if (req.headers.range) {
      const match = /^bytes=(\d+)-(\d+)$/.exec(req.headers.range);
      if (!match) { res.writeHead(416); res.end(); return; }
      first = Number(match[1]); last = Math.min(Number(match[2]), last);
      res.setHeader('Content-Range', `bytes ${first}-${last}/${bytes.length}`);
    }
    res.writeHead(req.headers.range ? 206 : 200, { ETag: '"unix-fixture-v1"', 'Accept-Ranges': 'bytes', 'Content-Length': last - first + 1 });
    res.end(bytes.subarray(first, last + 1));
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const service = spawn(join(binaries, 'stratafetch'), ['serve'], { env, stdio: ['ignore', 'ignore', 'pipe'] });
  let serviceError = ''; service.stderr.on('data', data => { serviceError += data; });
  const list = async () => JSON.parse((await exec(join(binaries, 'stratafetch'), ['--require-service', '--json', 'list'], { env, timeout: 10000 })).stdout);
  try {
    let ready = false;
    for (let attempt = 0; attempt < 100; attempt++) {
      try { const reply = await list(); assert.equal(reply.error, null); ready = true; break; }
      catch { assert.equal(service.exitCode, null, serviceError); await delay(20); }
    }
    assert.ok(ready, 'IPC must become ready');
    const ping = await native(env, origin, { action: 'ping' });
    assert.equal(ping.code, 0, ping.stderr);
    assert.equal(ping.stdout.readUInt32LE(), ping.stdout.length - 4);
    assert.equal(JSON.parse(ping.stdout.subarray(4)).ok, true);
    const rejected = await native(env, `chrome-extension://${'b'.repeat(32)}/`, { action: 'ping' });
    assert.equal(rejected.code, 1); assert.equal(rejected.stdout.length, 0);
    const added = await native(env, origin, { action: 'add', url: `http://127.0.0.1:${server.address().port}/sample.bin` });
    assert.equal(added.code, 0, added.stderr);
    assert.equal(added.stdout.readUInt32LE(), added.stdout.length - 4);
    const reply = JSON.parse(added.stdout.subarray(4)); assert.equal(reply.ok, true);
    let job;
    for (let attempt = 0; attempt < 100; attempt++) {
      job = (await list()).jobs.find(job => job.id === reply.download.id);
      if (job.status === 'Completed') break;
      assert.notEqual(job.status, 'Failed', job.error); await delay(20);
    }
    assert.equal(job.status, 'Completed'); assert.deepEqual(await readFile(job.destination), bytes);
    assert.equal(job.sha256, createHash('sha256').update(bytes).digest('hex'));
    await exec(join(binaries, 'stratafetch'), ['--require-service', 'verify', job.id], { env, timeout: 10000 });
  } finally {
    await stop(service);
    await new Promise(resolve => server.close(resolve));
    await rm(folder, { recursive: true, force: true });
  }
});
