const api = typeof browser !== "undefined" ? browser : chrome;
const pending = new Set();
let recovery;
let captureQueue = Promise.resolve();
function nativeMessage(payload) {
  return payload.action === "capture" ? queuedCapture(() => payload) : nativeRequest(payload);
}
function queuedCapture(makePayload) {
  // Pause downloads immediately, but ask about one file at a time. Re-read
  // queued downloads when their turn arrives, after any browser/user changes.
  const result = captureQueue.then(makePayload).then(nativeRequest);
  captureQueue = result.catch(() => {});
  return result;
}
function nativeRequest(payload) {
  if (!api.runtime.connectNative) return api.runtime.sendNativeMessage("org.stratafetch.host", payload);
  return new Promise((resolve, reject) => {
    const port = api.runtime.connectNative("org.stratafetch.host");
    let answered = false;
    port.onMessage.addListener(reply => { answered = true; resolve(reply); port.disconnect(); });
    port.onDisconnect.addListener(() => {
      const error = api.runtime.lastError;
      if (!answered) reject(new Error(error?.message || "Native host disconnected"));
    });
    try { port.postMessage(payload); }
    catch (error) { answered = true; port.disconnect(); reject(error); }
  });
}
api.runtime.onInstalled.addListener(async () => {
  await api.contextMenus.removeAll();
  api.contextMenus.create({ id: "stratafetch", title: "Download with StrataFetch", contexts: ["link"] });
  const settings = await api.storage.local.get({ capture: true });
  await api.storage.local.set({ capture: settings.capture });
});
api.contextMenus.onClicked.addListener(async info => {
  if (info.menuItemId !== "stratafetch") return;
  try {
    const response = await nativeMessage({ action: "capture", url: info.linkUrl });
    await showResult(response.ok, response.declined ? "Download declined" : response.ok ? "Download added to StrataFetch" : response.error);
  } catch { await showResult(false, "StrataFetch is unavailable. Open Settings → Browser integration in the app."); }
});
function eligible(item, ownedPause = false) {
  if (!item || item.incognito || item.byExtensionId || item.state !== "in_progress" || (item.paused && !ownedPause)) return false;
  if (item.danger && item.danger !== "safe") return false;
  const url = item.finalUrl || item.url;
  if (!/^https?:\/\//i.test(url || "")) return false;
  try { const parsed = new URL(url); if (parsed.username || parsed.password) return false; } catch { return false; }
  if (/^(text\/html|application\/x-xpinstall|application\/x-chrome-extension)(;|$)/i.test(item.mime || "")) return false;
  // Local browser-extension packages and generated page exports stay in the browser.
  return !/\.(crx|xpi|html?|mhtml)(?:$|[?#])/i.test(url) && !/\.(crx|xpi|html?|mhtml)$/i.test(item.filename || "");
}
function excluded(url, rules) {
  try {
    const host = new URL(url).hostname.toLowerCase().replace(/\.$/, "");
    return Array.isArray(rules) && rules.some(rule => typeof rule === "string" && rule && (host === rule || host.endsWith("." + rule)));
  } catch { return true; }
}
api.runtime.onMessage.addListener((message, _sender, respond) => {
  if (!message || !["capture", "ping"].includes(message.action)) return false;
  const payload = message.action === "ping" ? {action: "ping"} : {action: "capture", url: message.url};
  // The background owns the native port even when opening the confirmation
  // closes the extension popup.
  const reply = value => { try { respond(value); } catch { /* The popup may have closed. */ } };
  nativeMessage(payload).then(reply, () => reply({ok: false, error: "Native host unavailable. Finish local setup in the StrataFetch app."}));
  return true;
});
api.downloads.onCreated.addListener(item => intercept(item));
async function intercept(item) {
  if (!eligible(item) || pending.has(item.id)) return;
  pending.add(item.id);
  let paused = false;
  let accepted;
  try {
    const settings = await api.storage.local.get({ capture: true, excludedDomains: [] });
    if (!settings.capture || excluded(item.finalUrl || item.url, settings.excludedDomains)) return;
    await api.storage.local.set({["capture_pending_" + item.id]: {startTime: String(item.startTime || ""), timestamp: Date.now()}});
    await api.downloads.pause(item.id);
    paused = true;
    const reply = await queuedCapture(async () => {
      const [current] = await api.downloads.search({ id: item.id });
      const settings = await api.storage.local.get({capture: true, excludedDomains: []});
      if (!eligible(current, true) || current.startTime !== item.startTime || !current.paused) {
        throw new Error("Browser download changed before handoff");
      }
      if (!settings.capture || excluded(current.finalUrl || current.url, settings.excludedDomains)) throw new Error("Automatic capture disabled for this download");
      const payload = { action: "capture", url: current.finalUrl || current.url };
      const filename = (current.filename || item.filename || "").split(/[\\/]/).pop();
      if (filename) payload.name = filename;
      if (Number.isSafeInteger(current.totalBytes) && current.totalBytes >= 0) payload.size = current.totalBytes;
      if (current.mime) payload.mime = current.mime;
      return payload;
    });
    if (!reply?.ok) {
      if (!reply?.declined) await showResult(false, reply?.error || "Handoff failed; keeping the browser download.");
      return;
    }
    accepted = reply.download?.id;
    if (typeof accepted !== "string" || !accepted) { accepted = undefined; throw new Error("Native host did not acknowledge the download"); }
    await api.storage.local.set({["capture_pending_" + item.id]: {startTime: item.startTime, timestamp: Date.now(), nativeId: accepted}});
    const [latest] = await api.downloads.search({ id: item.id });
    if (!eligible(latest, true) || latest.startTime !== item.startTime || !latest.paused) {
      throw new Error("Browser download changed during handoff");
    }
    await api.downloads.cancel(item.id);
    paused = false;
    accepted = undefined;
    await showResult(true, "Download moved to StrataFetch");
  } catch {
    await showResult(false, "Handoff failed; the browser download is being resumed.");
  } finally {
    if (accepted) {
      try { if (!(await nativeMessage({ action: "cancel", id: accepted }))?.ok) throw new Error("Rollback rejected"); }
      catch { await showResult(false, "Cannot stop the StrataFetch copy. Check the app before retrying."); }
    }
    let recovered = true;
    if (paused) {
      try {
        const [current] = await api.downloads.search({id: item.id});
        if (current?.paused && current.state === "in_progress" && current.startTime === item.startTime) await api.downloads.resume(item.id);
      }
      catch { recovered = false; await showResult(false, "Use Resume in the browser downloads panel to continue."); }
    }
    if (recovered) {
      try { await api.storage.local.remove("capture_pending_" + item.id); } catch { /* Recovery retries stale bookkeeping on next start. */ }
    }
    pending.delete(item.id);
  }
}
async function showResult(ok, message) {
  await Promise.allSettled([
    api.action.setBadgeText({ text: ok ? "+" : "!" }),
    api.action.setBadgeBackgroundColor({ color: ok ? "#237d67" : "#a33030" }),
    api.action.setTitle({ title: `StrataFetch: ${String(message || "Download rejected").slice(0, 240)}` }),
  ]);
}


function recoverBrowserDownloads() {
  if (!recovery) recovery = performRecovery().finally(() => { recovery = undefined; });
  return recovery;
}
async function performRecovery() {
  const stored = await api.storage.local.get(null);
  for (const [key, record] of Object.entries(stored)) {
    if (!/^capture_pending_\d+$/.test(key)) continue;
    const id = Number(key.slice("capture_pending_".length));
    if (pending.has(id)) continue;
    let recovered = false;
    try {
      const [item] = await api.downloads.search({id});
      if (item?.paused && item.state === "in_progress" && record?.startTime && String(item.startTime) === record.startTime) {
        if (typeof record.nativeId === "string") {
          try { if (!(await nativeMessage({action: "cancel", id: record.nativeId}))?.ok) throw new Error("Rollback rejected"); }
          catch { await showResult(false, "Check the StrataFetch copy; the browser download will resume."); }
        }
        await api.downloads.resume(id);
        await showResult(false, "An interrupted handoff was resumed in the browser.");
      }
      recovered = true;
    } catch { await showResult(false, "An interrupted handoff needs Resume in the browser downloads panel."); }
    if (recovered) await api.storage.local.remove(key);
  }
}
api.runtime.onStartup.addListener(() => recoverBrowserDownloads().catch(() => {}));
recoverBrowserDownloads().catch(() => {});
