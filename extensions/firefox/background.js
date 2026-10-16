const api = typeof browser !== "undefined" ? browser : chrome;
const pending = new Set();
let recovery;
let captureQueue = Promise.resolve();
let eventWrites = Promise.resolve();
function recordEvent(event, item, nativeId) {
  const entry = {event, timestamp: Date.now(), downloadId: item.id};
  if (item.state) entry.state = String(item.state).slice(0, 32);
  if (typeof item.paused === "boolean") entry.paused = item.paused;
  if (item.danger) entry.safety = String(item.danger).slice(0, 32);
  if (nativeId) entry.nativeId = String(nativeId).slice(0, 64);
  // Only the latest 20 state transitions; no URLs, names, paths or page contents.
  eventWrites = eventWrites.catch(() => {}).then(async () => {
    const saved = await api.storage.local.get({handoffEvents: []});
    const previous = Array.isArray(saved.handoffEvents) ? saved.handoffEvents.slice(-19) : [];
    await api.storage.local.set({handoffEvents: [...previous, entry]});
  }).catch(() => {});
}
async function waitForBrowserCancellation(original) {
  for (let attempt = 0; attempt < 10; attempt++) {
    const [current] = await api.downloads.search({id: original.id});
    if (!current) return;
    if (!sameDownload(current, original)) throw new Error("Download identity changed after cancellation");
    if (current.state === "interrupted") return;
    if (current.state === "complete") throw new Error("The browser copy finished before cancellation");
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  throw new Error("Browser cancellation was not confirmed");
}
function safeIssue(error) {
  return String(error?.message || error || "No error details provided")
    .replace(/https?:\/\/[^\s]+/gi, "[URL]")
    .replace(/[A-Z]:\\[^\r\n]+/gi, "[local path]")
    .slice(0, 220);
}
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
  } catch (error) { await showResult(false, "Browser bridge: " + safeIssue(error)); }
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
// Chrome can update startTime after onCreated. The persistent ID and original
// URL identify the live download; valid timestamp updates are expected.
function sameDownload(current, original) {
  if (!current || current.id !== original.id || current.url !== original.url) return false;
  if (current.startTime === original.startTime) return true;
  const before = Date.parse(original.startTime), after = Date.parse(current.startTime);
  return Number.isFinite(after) && (!original.startTime || (Number.isFinite(before)));
}
function handoffIssue(current, original, requirePaused = true) {
  if (!sameDownload(current, original)) return "Download identity changed";
  if (!eligible(current, true)) {
    // Only enum/boolean diagnostics: never record download URLs or filenames.
    if (current.state !== "in_progress") return "Download is " + current.state;
    if (current.danger && current.danger !== "safe") return "Browser safety status is " + current.danger;
    return "Download became private, extension-managed, or an unsupported file type";
  }
  return requirePaused && !current.paused ? "Browser download is no longer paused" : "";
}
async function waitForOwnedPause(original) {
  for (let attempt = 0; attempt < 10; attempt++) {
    const [current] = await api.downloads.search({id: original.id});
    const issue = handoffIssue(current, original, false);
    if (issue) throw new Error(issue);
    if (current.paused) return current;
    if (attempt === 0 || attempt === 9) recordEvent("pause_not_observed", current);
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  throw new Error("The browser did not confirm its pause in time");
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
  nativeMessage(payload).then(reply, error => reply({ok: false, error: "Browser bridge: " + safeIssue(error)}));
  return true;
});
// onCreated may precede target-name and MIME determination. Observe only
// downloads born while this extension is active, then capture once metadata is ready.
const candidates = new Map();
const inspecting = new Set();
const inspectAgain = new Set();
const attempted = new Set();
const CANDIDATE_AGE = 5 * 60 * 1000;
async function urlFingerprint(url) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(url));
  return Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2, "0")).join("");
}
function candidateKey(id) { return "capture_candidate_" + id; }
async function forgetCandidate(id) {
  candidates.delete(id);
  await api.storage.local.remove(candidateKey(id));
}
async function watchDownload(item) {
  if (!eligible(item) || pending.has(item.id) || attempted.has(item.id)) return;
  const settings = await api.storage.local.get({capture: true, excludedDomains: []});
  if (!settings.capture || excluded(item.finalUrl || item.url, settings.excludedDomains) || candidates.has(item.id)) return;
  recordEvent("observed", item);
  const record = {observedAt: Date.now(), startTime: item.startTime, urlHash: await urlFingerprint(item.url)};
  if (candidates.has(item.id) || pending.has(item.id) || attempted.has(item.id)) return;
  candidates.set(item.id, record);
  await api.storage.local.set({[candidateKey(item.id)]: record});
  return inspectDownload(item.id);
}
async function inspectDownload(id) {
  if (pending.has(id) || attempted.has(id)) return;
  if (inspecting.has(id)) { inspectAgain.add(id); return; }
  inspecting.add(id);
  try {
    const record = candidates.get(id) || (await api.storage.local.get({[candidateKey(id)]: null}))[candidateKey(id)];
    if (!record) return;
    if (!Number.isFinite(record.observedAt) || Date.now() - record.observedAt > CANDIDATE_AGE) { await forgetCandidate(id); return; }
    const [current] = await api.downloads.search({id});
    if (!current || current.state !== "in_progress" || current.incognito || current.byExtensionId || current.paused) { await forgetCandidate(id); return; }
    if (await urlFingerprint(current.url) !== record.urlHash || !sameDownload(current, {id, url: current.url, startTime: record.startTime})) {
      await forgetCandidate(id);
      await showResult(false, "Download identity changed before metadata became ready. Keeping it in the browser.");
      return;
    }
    const settings = await api.storage.local.get({capture: true, excludedDomains: []});
    if (!settings.capture || excluded(current.finalUrl || current.url, settings.excludedDomains)) { await forgetCandidate(id); return; }
    // An empty target name is a normal early Chrome snapshot, not a rejection.
    if (!current.filename) return;
    if (!eligible(current)) { await forgetCandidate(id); return; }
    recordEvent("metadata_ready", current);
    attempted.add(id);
    if (attempted.size > 2048) attempted.delete(attempted.values().next().value);
    await forgetCandidate(id);
    return await intercept(current);
  } catch (error) {
    await showResult(false, "Observing browser download: " + safeIssue(error));
  } finally {
    inspecting.delete(id);
    if (inspectAgain.delete(id)) await inspectDownload(id);
  }
}
api.downloads.onCreated.addListener(item => watchDownload(item));
api.downloads.onChanged.addListener(delta => {
  if (pending.has(delta.id) && delta.paused) {
    recordEvent("browser_pause_changed", {id: delta.id, paused: delta.paused.current});
  }
  return inspectDownload(delta.id);
});

async function intercept(item) {
  if (!eligible(item) || pending.has(item.id)) return;
  pending.add(item.id);
  let paused = false;
  let accepted;
  let ownedStartTime = String(item.startTime || "");
  let phase = "Reading capture settings";
  try {
    const settings = await api.storage.local.get({ capture: true, excludedDomains: [] });
    if (!settings.capture || excluded(item.finalUrl || item.url, settings.excludedDomains)) return;
    await api.storage.local.set({["capture_pending_" + item.id]: {startTime: String(item.startTime || ""), timestamp: Date.now()}});
    phase = "Pausing browser download";
    recordEvent("pause_requested", item);
    await api.downloads.pause(item.id);
    paused = true;
    phase = "Checking browser download";
    const pausedItem = await waitForOwnedPause(item);
    ownedStartTime = String(pausedItem.startTime || "");
    recordEvent("browser_paused", pausedItem);
    await api.storage.local.set({["capture_pending_" + item.id]: {startTime: ownedStartTime, timestamp: Date.now()}});
    const reply = await queuedCapture(async () => {
      const [current] = await api.downloads.search({ id: item.id });
      const settings = await api.storage.local.get({capture: true, excludedDomains: []});
      const issue = handoffIssue(current, item);
      if (issue) throw new Error(issue);
      ownedStartTime = String(current.startTime || "");
      await api.storage.local.set({["capture_pending_" + item.id]: {startTime: ownedStartTime, timestamp: Date.now()}});
      if (!settings.capture || excluded(current.finalUrl || current.url, settings.excludedDomains)) throw new Error("Automatic capture disabled for this download");
      const payload = { action: "capture", url: current.finalUrl || current.url };
      const filename = (current.filename || item.filename || "").split(/[\\/]/).pop();
      if (filename) payload.name = filename;
      if (Number.isSafeInteger(current.totalBytes) && (current.totalBytes > 0 || (current.totalBytes === 0 && current.fileSize === 0))) payload.size = current.totalBytes;
      if (current.mime) payload.mime = current.mime;
      phase = "Opening browser bridge";
      recordEvent("confirmation_requested", current);
      return payload;
    });
    if (!reply?.ok) {
      recordEvent(reply?.declined ? "declined" : "native_rejected", item);
      if (!reply?.declined) await showResult(false, reply?.error || "Handoff failed; keeping the browser download.");
      return;
    }
    accepted = reply.download?.id;
    if (typeof accepted !== "string" || !accepted) { accepted = undefined; throw new Error("Native host did not acknowledge the download"); }
    await api.storage.local.set({["capture_pending_" + item.id]: {startTime: ownedStartTime, timestamp: Date.now(), nativeId: accepted}});
    recordEvent("native_acknowledged", {id: item.id}, accepted);
    const [latest] = await api.downloads.search({ id: item.id });
    const latestIssue = handoffIssue(latest, item);
    if (latestIssue) throw new Error(latestIssue);
    phase = "Cancelling browser copy after acknowledgment";
    await api.downloads.cancel(item.id);
    await waitForBrowserCancellation(item);
    recordEvent("browser_cancelled", {...latest, state: "interrupted", paused: false}, accepted);
    paused = false;
    accepted = undefined;
    await showResult(true, "Download moved to StrataFetch");
  } catch (error) {
    recordEvent("handoff_failed", {id: item.id}, accepted);
    await showResult(false, phase + ": " + safeIssue(error) + ". Keeping the download in your browser.");
  } finally {
    let recovered = true;
    if (accepted) {
      try {
        if (!(await nativeMessage({ action: "cancel", id: accepted }))?.ok) throw new Error("Rollback rejected");
        recordEvent("native_cancelled", item, accepted);
      }
      catch {
        recovered = false;
        recordEvent("native_cancel_unconfirmed", item, accepted);
        await showResult(false, "StrataFetch cancellation is unconfirmed. The browser copy stays paused; check the app before resuming.");
      }
    }
    if (paused && recovered) {
      try {
        const [current] = await api.downloads.search({id: item.id});
        if (current?.paused && current.state === "in_progress" && sameDownload(current, item)) {
          await api.downloads.resume(item.id);
          recordEvent("browser_resumed", {...current, paused: false});
        }
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
  const detail = safeIssue(message || "Download rejected");
  await Promise.allSettled([
    ok ? api.storage.local.remove("lastHandoffError") : api.storage.local.set({lastHandoffError: {message: detail, timestamp: Date.now()}}),
    api.action.setBadgeText({ text: ok ? "+" : "!" }),
    api.action.setBadgeBackgroundColor({ color: ok ? "#237d67" : "#a33030" }),
    api.action.setTitle({ title: `StrataFetch: ${detail}` }),
  ]);
}


function recoverBrowserDownloads() {
  if (!recovery) recovery = performRecovery().finally(() => { recovery = undefined; });
  return recovery;
}
async function performRecovery() {
  const stored = await api.storage.local.get(null);
  for (const [key, record] of Object.entries(stored)) {
    if (/^capture_candidate_\d+$/.test(key)) {
      if (!Number.isFinite(record?.observedAt) || Date.now() - record.observedAt > CANDIDATE_AGE) await api.storage.local.remove(key);
      continue;
    }
    if (!/^capture_pending_\d+$/.test(key)) continue;
    const id = Number(key.slice("capture_pending_".length));
    if (pending.has(id)) continue;
    let recovered = false;
    try {
      const [item] = await api.downloads.search({id});
      if (item?.paused && item.state === "in_progress" && record?.startTime && String(item.startTime) === record.startTime) {
        if (typeof record.nativeId === "string") {
          try { if (!(await nativeMessage({action: "cancel", id: record.nativeId}))?.ok) throw new Error("Rollback rejected"); }
          catch { throw new Error("StrataFetch cancellation is unconfirmed"); }
        }
        await api.downloads.resume(id);
        await showResult(false, "An interrupted handoff was resumed in the browser.");
      }
      recovered = true;
    } catch { await showResult(false, "Handoff recovery is incomplete. Check StrataFetch before resuming the paused browser copy."); }
    if (recovered) await api.storage.local.remove(key);
  }
}
api.runtime.onStartup.addListener(() => recoverBrowserDownloads().catch(() => {}));
recoverBrowserDownloads().catch(() => {});
