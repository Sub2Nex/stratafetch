import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

for (const [folder, namespace] of [['chromium', 'chrome'], ['firefox', 'browser']]) {
  for (const result of ['success', 'rejected', 'unavailable']) {
    test(`${folder}: context handoff ${result} reports visible status`, async () => {
      let installed, clicked;
      const calls = [];
      const api = {
        runtime: { onInstalled: { addListener(fn) { installed = fn; } },
          async sendNativeMessage(host, payload) {
            calls.push({ host, payload });
            if (result === 'unavailable') throw new Error('Native host not found');
            return result === 'success' ? { ok: true } : { ok: false, error: 'Only HTTP and HTTPS URLs are supported' };
          } },
        contextMenus: { create(value) { calls.push(value); }, onClicked: { addListener(fn) { clicked = fn; } } },
        action: { async setBadgeText(value) { calls.push(value); }, async setBadgeBackgroundColor(value) { calls.push(value); }, async setTitle(value) { calls.push(value); } },
      };
      runInNewContext(readFileSync(new URL(`../extensions/${folder}/background.js`, import.meta.url), 'utf8'), { [namespace]: api });
      installed();
      assert.deepEqual(Array.from(calls[0].contexts), ['link']);
      await clicked({ menuItemId: 'stratafetch', linkUrl: 'https://example.org/file.zip' });
      assert.equal(calls[1].host, 'org.stratafetch.host');
      assert.equal(calls[1].payload.action, 'add');
      assert.equal(calls[1].payload.url, 'https://example.org/file.zip');
      assert.deepEqual(Object.keys(calls[1].payload).sort(), ['action','url']);
      assert.equal(calls[2].text, result === 'success' ? '+' : '!');
      assert.match(calls[4].title, result === 'success' ? /added/ : result === 'rejected' ? /HTTP/ : /unavailable/);
      const before = calls.length;
      await clicked({ menuItemId: 'unrelated', linkUrl: 'https://example.org' });
      assert.equal(calls.length, before);
    });
  }
  test(`${folder}: minimal permissions and no content scripts`, () => {
    const manifest = JSON.parse(readFileSync(new URL(`../extensions/${folder}/manifest.json`, import.meta.url)));
    assert.deepEqual(manifest.permissions.slice().sort(), ['activeTab','contextMenus','nativeMessaging']);
    assert.equal(manifest.host_permissions, undefined);
    assert.equal(manifest.content_scripts, undefined);
  });
  for (const result of ['empty', 'restricted', 'error']) {
    test(`${folder}: current page ${result} shows a useful error`, async () => {
      const elements = Object.fromEntries(['status', 'send', 'current', 'url'].map(id => [id, { value: 'https://example.org/previous.zip' }]));
      const api = {
        runtime: { async sendNativeMessage() { return { ok: true, active: 0 }; } },
        tabs: { async query() {
          if (result === 'error') throw new Error('Tab access denied');
          return result === 'empty' ? [] : [{ id: 1 }];
        } },
      };
      runInNewContext(readFileSync(new URL(`../extensions/${folder}/popup.js`, import.meta.url), 'utf8'), {
        [namespace]: api, document: { getElementById(id) { return elements[id]; } },
      });
      // Let the initial native-host ping finish before selecting a tab.
      await Promise.resolve();
      await elements.current.onclick();
      assert.equal(elements.url.value, '');
      assert.match(elements.status.textContent, /cannot read.*paste/i);
      assert.equal(elements.current.disabled, false);
    });
  }
}
for (const [folder, namespace] of [['chromium', 'chrome'], ['firefox', 'browser']]) {
  test(`${folder}: late startup ping preserves the current-tab error`, async () => {
    const elements = Object.fromEntries(['status', 'send', 'current', 'url'].map(id => [id, { value: 'previous URL' }]));
    let resolvePing;
    const api = {
      runtime: { sendNativeMessage() { return new Promise(resolve => { resolvePing = resolve; }); } },
      tabs: { async query() { throw new Error('Tab access denied'); } },
    };
    runInNewContext(readFileSync(new URL(`../extensions/${folder}/popup.js`, import.meta.url), 'utf8'), {
      [namespace]: api, document: { getElementById(id) { return elements[id]; } },
    });
    await elements.current.onclick();
    const error = elements.status.textContent;
    resolvePing({ ok: true, active: 0 });
    await Promise.resolve(); await Promise.resolve();
    assert.equal(elements.status.textContent, error);
    assert.match(error, /cannot read/i);
  });
  test(`${folder}: badge API errors do not become unhandled handoff failures`, async () => {
    let clicked;
    let nativeCalls = 0;
    const api = {
      runtime: { onInstalled: { addListener() {} }, async sendNativeMessage() { nativeCalls++; return { ok: true }; } },
      contextMenus: { onClicked: { addListener(fn) { clicked = fn; } } },
      action: { async setBadgeText() { throw new Error('Action unavailable'); }, async setBadgeBackgroundColor() {}, async setTitle() {} },
    };
    runInNewContext(readFileSync(new URL(`../extensions/${folder}/background.js`, import.meta.url), 'utf8'), { [namespace]: api });
    await assert.doesNotReject(clicked({ menuItemId: 'stratafetch', linkUrl: 'https://example.org/file' }));
    assert.equal(nativeCalls, 1);
  });
}
