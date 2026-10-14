// Run after a release build. Reject an untrusted loopback certificate without downloading data.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdtemp, readFile, rm, access } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'node:https';
const exec = promisify(execFile);
const root = fileURLToPath(new URL('../', import.meta.url));
test('release engine rejects an untrusted HTTPS certificate before receiving payload', { timeout: 30000 }, async () => {
  const folder = await mkdtemp(join(tmpdir(), 'stratafetch-tls-'));
  let server;
  try {
    const key = join(folder, 'fixture.key'), certificate = join(folder, 'fixture.crt');
    await exec('openssl', ['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', key, '-out', certificate, '-days', '1', '-subj', '/CN=localhost', '-addext', 'subjectAltName=DNS:localhost,IP:127.0.0.1']);
    let payloadRequests = 0;
    server = createServer({ key: await readFile(key), cert: await readFile(certificate) }, (request, response) => { payloadRequests++; response.end('untrusted payload'); });
    server.on('tlsClientError', () => {});
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const binary = join(resolve(process.env.STRATAFETCH_BIN_DIR || join(root, 'target/release')), process.platform === 'win32' ? 'stratafetch.exe' : 'stratafetch');
    const destination = join(folder, 'downloads');
    let failure;
    try { await exec(binary, ['--state', join(folder, 'state'), '--json', 'add', `https://127.0.0.1:${server.address().port}/untrusted.bin`, '--directory', destination], { timeout: 20000 }); }
    catch (error) { failure = error; }
    assert.ok(failure, 'untrusted TLS must fail');
    assert.equal(failure.code, 1);
    const reply = JSON.parse(failure.stdout);
    assert.equal(reply.jobs.length, 1);
    assert.equal(reply.jobs[0].status, 'Failed');
    assert.equal(reply.jobs[0].downloaded, 0);
    assert.equal(payloadRequests, 0);
    await assert.rejects(access(join(destination, 'untrusted.bin')));
  } finally {
    if (server) await new Promise(resolve => server.close(resolve));
    await rm(folder, { recursive: true, force: true });
  }
});
