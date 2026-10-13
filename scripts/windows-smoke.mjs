// Real executables, local HTTP, OS IPC, native frames, process-kill recovery and native framebuffer.
// Run after release binaries and the fixture_server example are built. Test data stays in .smoke.
import { spawn, execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { createHash } from 'node:crypto';
import assert from 'node:assert/strict';
const root = resolve(import.meta.dirname, '..');
const directory = join(root, '.smoke', `phase2-${Date.now()}`);
mkdirSync(directory, { recursive: true });
const cli = join(root, 'target/release/stratafetch.exe');
const host = join(root, 'target/release/stratafetch-browser-host.exe');
const state = join(directory, 'state');
const output = join(directory, 'downloads');
mkdirSync(state); mkdirSync(output);
const env = { ...process.env, STRATAFETCH_STATE_DIR: state, STRATAFETCH_DOWNLOAD_DIR: output };
const origin = `chrome-extension://${'a'.repeat(32)}/`;
writeFileSync(join(state, 'browser-origins.json'), JSON.stringify([origin]));
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
function run(args) { return JSON.parse(execFileSync(cli, ['--state', state, '--json', ...args], { env, windowsHide: true, timeout: 30000 }).toString()); }
function waitExit(child, timeout = 30000) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => { child.kill(); reject(new Error('Process timeout')); }, timeout);
    child.once('error', error => { clearTimeout(timer); reject(error); });
    child.once('exit', code => { clearTimeout(timer); resolve(code); });
  });
}
async function native(payload, caller = origin) {
  const child = spawn(host, [caller], { env, windowsHide: true });
  let stdout = Buffer.alloc(0), stderr = '';
  child.stdout.on('data', data => { stdout = Buffer.concat([stdout, data]); });
  child.stderr.on('data', data => { stderr += data; });
  const bytes = Buffer.from(JSON.stringify(payload));
  const prefix = Buffer.alloc(4); prefix.writeUInt32LE(bytes.length);
  child.stdin.end(Buffer.concat([prefix, bytes]));
  const code = await waitExit(child);
  if (caller !== origin) { assert.notEqual(code, 0); return { rejected: true }; }
  assert.equal(code, 0, stderr);
  assert.equal(stdout.readUInt32LE(0), stdout.length - 4);
  return JSON.parse(stdout.subarray(4));
}
const fixture = spawn(join(root, 'target/release/examples/fixture_server.exe'), [], { windowsHide: true });
let fixtureError = '';
fixture.stderr.on('data', bytes => { fixtureError += bytes; });
let service;
try {
  for (let i = 0; i < 100; i++) {
    if (fixture.exitCode !== null) throw new Error(`Fixture exited: ${fixtureError}`);
    try { const response = await fetch('http://127.0.0.1:18765/fixture.bin', { headers: { Range: 'bytes=0-0' } }); await response.arrayBuffer(); break; } catch { await sleep(50); if(i===99) throw new Error('Fixture unavailable'); }
  }
  run(['settings', '--connections', '4', '--concurrent-jobs', '1', '--global-bytes-per-second', '1048576']);
  service = spawn(cli, ['--state', state, 'serve'], { env, windowsHide: true });
  for (let i = 0; i < 100; i++) {
    try { run(['--require-service', 'list']); break; } catch { await sleep(50); if(i===99) throw new Error('Service unavailable'); }
  }
  const ping = await native({ action: 'ping' }); assert.equal(ping.ok, true);
  await native({ action: 'ping' }, 'chrome-extension://unregistered/');
  const added = await native({ action: 'add', url: 'http://127.0.0.1:18765/fixture.bin?slow=1' });
  assert.equal(added.ok, true); assert.ok(added.download.id);
  await sleep(1500);
  const before = run(['--require-service', 'status', added.download.id]).jobs[0];
  assert.equal(before.status, 'Downloading'); assert.ok(before.downloaded > 0 && before.downloaded < 8388608);
  const killed = waitExit(service); service.kill(); await killed; service = undefined;
  const recovered = run(['status', added.download.id]).jobs[0];
  assert.equal(recovered.status, 'Paused'); assert.ok(recovered.downloaded > 0);
  run(['settings', '--connections', '4', '--concurrent-jobs', '1']);
  const completed = run(['resume', added.download.id]).jobs[0];
  assert.equal(completed.status, 'Completed'); assert.equal(completed.downloaded, 8388608);
  const expected = 'bdf23837181f5808331800c1ae2b4f7d7a839536b10d58491471c50dde23833a';
  assert.equal(createHash('sha256').update(readFileSync(completed.destination)).digest('hex'), expected);
  run(['verify', added.download.id, expected]);
  const unavailable = await native({ action: 'ping' }); assert.equal(unavailable.ok, false);
  const screenshot = join(root, 'docs/screenshot-phase2.png');
  const desktop = spawn(join(root, 'target/release/stratafetch-desktop.exe'), [], { env: { ...env, STRATAFETCH_SMOKE_OUTPUT: screenshot }, windowsHide: false });
  assert.equal(await waitExit(desktop), 0);
  assert.ok(readFileSync(screenshot).length > 1000);
  const report = { ping, added, pausedBytesAfterKill: recovered.downloaded, completedBytes: completed.downloaded, sha256: expected, unavailableErrorReported: true, nativeDesktopExit: 0, screenshot };
  writeFileSync(join(directory, 'result.json'), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ directory, ...report }, null, 2));
} finally {
  service?.kill(); fixture.kill();
}
