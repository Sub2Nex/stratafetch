if (typeof importScripts === "function" && typeof StrataLocale === "undefined") importScripts("locale.js");
const api = typeof browser !== "undefined" ? browser : chrome;
const locale = globalThis.StrataLocale || {t: value => value, setLanguage() {}};
const pending = new Set();
const captures = new Map();
let recovery;
let captureQueue = Promise.resolve();
let eventWrites = Promise.resolve();
// Observe methods only; never request cookies, authorization headers or bodies.
// MV3 webRequest cannot block responses, but tells us whether a cancelled
// download can be restarted as a GET instead of replaying a form submission.
const requests = new Map();
function observeRequest(details) {
  const now = Date.now();
  for (const [id, value] of requests) if (now - value.time > 60000) requests.delete(id);
  if (details.incognito || !/^https?:\/\//i.test(details.url)) return;
  const previous = requests.get(details.requestId);
  requests.set(details.requestId, {time: now, get: details.method === "GET" && (!previous || previous.get),
    urls: [...new Set([...(previous?.urls || []), details.url])].slice(-12)});
  if (requests.size > 512) requests.delete(requests.keys().next().value);
}
if (api.webRequest?.onBeforeRequest) {
  api.webRequest.onBeforeRequest.addListener(observeRequest, {urls: ["http://*/*", "https://*/*"]});
}
function requestMethod(item) {
  const matches = [...requests.values()].filter(value => Date.now() - value.time <= 60000
    && (value.urls.includes(item.url) || value.urls.includes(item.finalUrl)));
  return !matches.length ? "unknown" : matches.every(value => value.get) ? "GET" : "other";
}
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
function queuedCapture(makePayload, beforeCommit) {
  // Stop browser transfers immediately, then ask about one file at a time.
  const result = captureQueue.then(makePayload).then(payload => nativeRequest(payload, beforeCommit));
  captureQueue = result.catch(() => {});
  return result;
}
function nativeRequest(payload, beforeCommit) {
  if (!api.runtime.connectNative) return api.runtime.sendNativeMessage("org.stratafetch.host", payload);
  return new Promise((resolve, reject) => {
    const port = api.runtime.connectNative("org.stratafetch.host");
    let answered = false;
    port.onMessage.addListener(reply => {
      if (reply?.ok && reply.confirmation === "accepted") {
        Promise.resolve().then(() => beforeCommit?.()).then(commit => {
          if (!answered) port.postMessage(commit || {action: "commitCapture"});
        }).catch(error => { if (!answered) { answered = true; reject(error); port.disconnect(); } });
        return;
      }
      answered = true; resolve(reply); port.disconnect();
    });
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
  const saved = await api.storage.local.get({appLanguage: null});
  locale.setLanguage(saved.appLanguage);
  api.contextMenus.create({ id: "stratafetch", title: locale.t("Download with StrataFetch"), contexts: ["link"] });
  const settings = await api.storage.local.get({ capture: true });
  await api.storage.local.set({ capture: settings.capture });
});
api.contextMenus.onClicked.addListener(async info => {
  if (info.menuItemId !== "stratafetch") return;
  try {
    const response = await nativeMessage({ action: "capture", url: info.linkUrl });
    await showResult(response.ok, response.declined ? "Download cancelled" : response.ok ? "Download added to StrataFetch" : response.error);
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
  if (!eligible(current?.state === "complete" ? {...current, state: "in_progress"} : current, true)) {
    // Only enum/boolean diagnostics: never record download URLs or filenames.
    if (current.state !== "in_progress") return "Download is " + current.state;
    if (current.danger && current.danger !== "safe") return "Browser safety status is " + current.danger;
    return "Download became private, extension-managed, or an unsupported file type";
  }
  return ""; // Pause ownership is enforced at commit, never gates the dialog.
}
function excluded(url, rules) {
  try {
    const host = new URL(url).hostname.toLowerCase().replace(/\.$/, "");
    return Array.isArray(rules) && rules.some(rule => typeof rule === "string" && rule && (host === rule || host.endsWith("." + rule)));
  } catch { return true; }
}
function matchesCaptureFilters(item, settings) {
  const minimum = settings.captureMinBytes ?? 0;
  const types = settings.captureExtensions ?? [];
  if (!Number.isSafeInteger(minimum) || minimum < 0 || minimum > 1099511627776 || !Array.isArray(types) || types.length > 100) return false;
  if (minimum && (!Number.isSafeInteger(item.totalBytes) || item.totalBytes < minimum)) return false;
  if (!types.length) return true;
  try {
    const name = (item.filename || decodeURIComponent(new URL(item.finalUrl || item.url).pathname)).split(/[\\/]/).pop().toLowerCase();
    return types.every(type => typeof type === "string" && /^[a-z0-9]+(?:\.[a-z0-9]+)*$/.test(type) && type.length <= 32)
      && types.some(type => name.endsWith("." + type));
  } catch { return false; }
}
api.runtime.onMessage.addListener((message, _sender, respond) => {
  if (!message || !["capture", "ping"].includes(message.action)) return false;
  const payload = message.action === "ping" ? {action: "ping"} : {action: "capture", url: message.url};
  // The background owns the native port even when opening the confirmation
  // closes the extension popup.
  const reply = value => {
    if (value?.language === "sv" || value?.language === "en") {
      locale.setLanguage(value.language);
      api.storage.local.set({appLanguage: value.language}).catch(() => {});
      if (api.contextMenus.update) Promise.resolve(api.contextMenus.update("stratafetch", {title: locale.t("Download with StrataFetch")})).catch(() => {});
    }
    try { respond(value); } catch { /* The popup may have closed. */ }
  };
  nativeMessage(payload).then(reply, error => reply({ok: false, error: "Browser bridge: " + safeIssue(error)}));
  return true;
});
// Claim each new ID once; cancellation and erasure precede native consent.
const attempted = new Set();
async function watchDownload(item) {
  if (!eligible(item) || pending.has(item.id) || attempted.has(item.id)) return;
  attempted.add(item.id);
  if (attempted.size > 2048) attempted.delete(attempted.values().next().value);
  recordEvent("observed", item);
  return intercept(item);
}
api.downloads.onCreated.addListener(item => watchDownload(item));
// Filename notification is a fallback when onCreated was missed.
// Do not delay Chrome's filename callback: this is not a network-blocking API.
if (api.downloads.onDeterminingFilename) {
  api.downloads.onDeterminingFilename.addListener((item, suggest) => {
    suggest();
    if (!eligible(item) || pending.has(item.id) || attempted.has(item.id)) return;
    watchDownload(item).catch(() => {});
  });
}
api.downloads.onChanged.addListener(delta => {
  const capture = captures.get(delta.id);
  if (capture && delta.danger?.current && delta.danger.current !== "safe") {
    capture.error = "Browser safety status is " + delta.danger.current;
  }
});

async function intercept(item) {
  await (recovery || startupRecovery);
  if (!eligible(item) || pending.has(item.id)) return;
  pending.add(item.id);
  const capture = {error: null};
  captures.set(item.id, capture);
  let stopped = false;
  let removed = false;
  let recorded = false;
  try {
    const settings = await api.storage.local.get({capture: true, excludedDomains: [], captureMinBytes: 0, captureExtensions: []});
    if (!settings.capture || excluded(item.finalUrl || item.url, settings.excludedDomains) || !matchesCaptureFilters(item, settings) || requestMethod(item) === "other") return;
    const [current] = await api.downloads.search({id: item.id});
    const issue = handoffIssue(current, item, false);
    if (issue) throw new Error(issue);
    // Snapshot metadata before erasing Chrome's entry. Never reinterpret the
    // missing entry as a failed handoff or restart it through the browser.
    const payload = {action: "capture", url: current.finalUrl || current.url};
    const name = (current.filename || item.filename || "").split(/[\\/]/).pop();
    if (name) payload.name = name;
    if (current.mime) payload.mime = current.mime;
    if (Number.isSafeInteger(current.totalBytes) && (current.totalBytes > 0 || current.fileSize === 0 && current.totalBytes === 0)) payload.size = current.totalBytes;
    await api.storage.local.set({["capture_pending_" + item.id]: {startTime: String(current.startTime || ""), timestamp: Date.now(), policy: "cancel"}});
    recorded = true;
    recordEvent("browser_stop_requested", current);
    if (current.state !== "complete") await api.downloads.cancel(item.id);
    const [final] = await api.downloads.search({id: item.id});
    if (final && !sameDownload(final, item)) throw new Error("Download identity changed after cancellation");
    if (capture.error) throw new Error(capture.error);
    if (final?.danger && final.danger !== "safe") throw new Error("Browser safety status is " + final.danger);
    if (final?.state === "complete") {
      // A tiny file can finish before Chrome processes cancel. Delete only the
      // newly intercepted browser-owned copy; Start will use the native app.
      await api.downloads.removeFile(item.id);
    } else await waitForBrowserCancellation(item);
    stopped = true;
    await api.downloads.erase({id: item.id});
    removed = true;
    recordEvent("browser_entry_removed", {id: item.id});
    const reply = await queuedCapture(async () => {
      if (capture.error) throw new Error(capture.error);
      recordEvent("confirmation_requested", {id: item.id});
      return payload;
    }, () => {
      if (!stopped || capture.error) throw new Error(capture.error || "Browser cancellation was not confirmed");
      recordEvent("native_commit_authorized", {id: item.id});
      return {...payload, action: "commitCapture"};
    });
    if (reply?.declined) {
      recordEvent("cancelled", {id: item.id});
      await showResult(true, "Download cancelled");
    } else if (reply?.ok && typeof reply.download?.id === "string" && reply.download.id) {
      recordEvent("native_acknowledged", {id: item.id}, reply.download.id);
      await showResult(true, "Download added to StrataFetch");
    } else throw new Error(reply?.error || "Native host did not acknowledge the download");
  } catch (error) {
    recordEvent("handoff_failed", {id: item.id});
    await showResult(false, "Browser bridge: " + safeIssue(error) + ". Download was not started. Repair browser integration and retry.");
  } finally {
    captures.delete(item.id);
    pending.delete(item.id);
    // Never resume or recreate a captured request, including on bridge failure.
    // Retain an unconfirmed stop for startup cleanup; native jobs stay owned by
    // the native app even if its final acknowledgment is lost.
    if (recorded && removed) await api.storage.local.remove("capture_pending_" + item.id).catch(() => {});
  }
}
async function showResult(ok, message) {
  const detail = safeIssue(message || "Download rejected");
  await Promise.allSettled([
    ok ? api.storage.local.remove("lastHandoffError") : api.storage.local.set({lastHandoffError: {message: detail, timestamp: Date.now()}}),
    api.action.setBadgeText({ text: ok ? "+" : "!" }),
    api.action.setBadgeBackgroundColor({ color: ok ? "#237d67" : "#a33030" }),
    api.action.setTitle({ title: `StrataFetch: ${locale.error ? locale.error(detail) : detail}` }),
  ]);
}


function recoverBrowserDownloads() {
  if (!recovery) recovery = performRecovery().finally(() => { recovery = undefined; });
  return recovery;
}
async function performRecovery() {
  const stored = await api.storage.local.get(null);
  for (const [key, record] of Object.entries(stored)) {
    if (/^capture_candidate_\d+$/.test(key)) { await api.storage.local.remove(key); continue; }
    if (!/^capture_pending_\d+$/.test(key)) continue;
    const id = Number(key.slice("capture_pending_".length));
    if (pending.has(id)) continue;
    try {
      const [item] = await api.downloads.search({id});
      if (pending.has(id)) continue;
      if (item && String(item.startTime || "") === record?.startTime) {
        if (item.state === "in_progress") {
          await api.downloads.cancel(id);
          await waitForBrowserCancellation(item);
        }
        // Never delete a completed file during crash recovery.
        if (item.state !== "complete") await api.downloads.erase({id});
      }
      await api.storage.local.remove(key);
    } catch { await showResult(false, "Download cleanup failed. Cancel the browser entry and retry after repairing integration."); }
  }
}
api.runtime.onStartup.addListener(() => recoverBrowserDownloads().catch(() => {}));
const startupRecovery = recoverBrowserDownloads().catch(() => {});
