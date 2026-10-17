const api = typeof browser !== "undefined" ? browser : chrome;
const pending = new Set();
const pauseLeases = new Map();
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
function queuedCapture(makePayload, beforeCommit) {
  // Pause downloads immediately, but ask about one file at a time. Re-read
  // queued downloads when their turn arrives, after any browser/user changes.
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
  if (!eligible(current?.state === "complete" ? {...current, state: "in_progress"} : current, true)) {
    // Only enum/boolean diagnostics: never record download URLs or filenames.
    if (current.state !== "in_progress") return "Download is " + current.state;
    if (current.danger && current.danger !== "safe") return "Browser safety status is " + current.danger;
    return "Download became private, extension-managed, or an unsupported file type";
  }
  return ""; // Pause ownership is enforced at commit, never gates the dialog.
}
async function waitForOwnedPause(original) {
  for (let attempt = 0; attempt < 10; attempt++) {
    const [current] = await api.downloads.search({id: original.id});
    const issue = handoffIssue(current, original, false);
    if (issue) throw new Error(issue);
    if (current.paused || current.state === "complete") return current;
    if (attempt === 0 || attempt === 9) recordEvent("pause_not_observed", current);
    await new Promise(resolve => setTimeout(resolve, 50));
  }
  throw new Error("The browser did not confirm its pause in time");
}
function ownPause(original) {
  let active = true, error, changes = 0, work = Promise.resolve();
  const ensure = () => {
    const next = work.then(async () => {
      if (!active) throw new Error("Browser handoff ended");
      const [current] = await api.downloads.search({id: original.id});
      const issue = handoffIssue(current, original, false);
      if (issue) throw new Error(issue);
      if (current.paused || current.state === "complete") return current;
      if (error) throw error;
      // Other download handlers (including security inspection) can resume
      // after our pause. Keep our own hold without accepting unsafe downloads
      // or interfering with a browser cancellation. The confirmation's Keep
      // in browser button releases this lease before resuming.
      if (++changes > 12) throw new Error("Another download handler repeatedly released the pause");
      recordEvent("pause_reacquired", current);
      try { await api.downloads.pause(original.id); }
      catch (failure) {
        const [latest] = await api.downloads.search({id: original.id});
        if (!handoffIssue(latest, original, false) && latest.state === "complete") return latest;
        throw failure;
      }
      return await waitForOwnedPause(original);
    });
    work = next.catch(failure => { error = failure; });
    return next;
  };
  return {ensure, active: () => active, release: async () => { active = false; await work; }};
}

async function restartBrowser(recipe) {
  // downloads.download uses the browser's normal request/security machinery.
  // No cookies/headers are read or copied by the extension.
  const options = {url: recipe.url, conflictAction: "uniquify"};
  if (recipe.name) options.filename = recipe.name;
  const id = await api.downloads.download(options);
  if (!Number.isSafeInteger(id)) throw new Error("Browser did not restart the download");
  recordEvent("browser_restarted", {id});
  return id;
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
// Event handlers claim a newly observed ID immediately. Opening the native
// dialog and acquiring the browser pause proceed independently.
const attempted = new Set();
async function watchDownload(item) {
  if (!eligible(item) || pending.has(item.id) || attempted.has(item.id)) return;
  attempted.add(item.id);
  if (attempted.size > 2048) attempted.delete(attempted.values().next().value);
  recordEvent("observed", item);
  return intercept(item);
}
api.downloads.onCreated.addListener(item => watchDownload(item));
// Chrome exposes the tentative filename before its later filename onChanged
// notification. Start the pause here instead of waiting for target-path updates.
// Do not delay Chrome's filename callback: this is not a network-blocking API.
if (api.downloads.onDeterminingFilename) {
  api.downloads.onDeterminingFilename.addListener((item, suggest) => {
    suggest();
    if (!eligible(item) || pending.has(item.id) || attempted.has(item.id)) return;
    watchDownload(item).catch(() => {});
  });
}
api.downloads.onChanged.addListener(delta => {
  if (pending.has(delta.id) && delta.paused) {
    recordEvent("browser_pause_changed", {id: delta.id, paused: delta.paused.current});
  }
  if (delta.paused?.current === false) pauseLeases.get(delta.id)?.ensure().catch(() => {});
  if (delta.totalBytes || delta.mime || delta.finalUrl) pauseLeases.get(delta.id)?.tryExclusive?.().catch(() => {});

});

async function intercept(item) {
  // Startup recovery must finish before a new handoff can claim a browser ID.
  // Otherwise recovery's async search can resume a download we just paused.
  await (recovery || startupRecovery);
  if (!eligible(item) || pending.has(item.id)) return;
  pending.add(item.id);
  let paused = false;
  let accepted;
  let ownedStartTime = String(item.startTime || "");
  let phase = "Reading capture settings";
  let lease;
  let exclusive;
  let exclusiveProbe;
  let cancelWork;
  let nativeOwns = false;
  const record = nativeId => ({startTime: ownedStartTime, timestamp: Date.now(),
    ...(exclusive ? {exclusive} : {}), ...(nativeId ? {nativeId} : {})});
  const checked = async () => {
    const [current] = await api.downloads.search({id: item.id});
    // Only our verified cancellation grants this exception. Browser security,
    // identity/private/type checks still apply to the live record.
    const issue = handoffIssue(exclusive && current?.state === "interrupted" && current.error === "USER_CANCELED"
      ? {...current, state: "in_progress", paused: true} : current, item, false);
    if (issue) throw new Error(issue);
    return current;
  };
  try {
    const settings = await api.storage.local.get({ capture: true, excludedDomains: [] });
    if (!settings.capture || excluded(item.finalUrl || item.url, settings.excludedDomains)) return;
    await api.storage.local.set({["capture_pending_" + item.id]: {startTime: String(item.startTime || ""), timestamp: Date.now()}});
    phase = "Opening browser bridge";
    recordEvent("pause_requested", item);
    paused = true;
    lease = ownPause(item);
    pauseLeases.set(item.id, lease);
    lease.tryExclusive = () => {
      if (exclusiveProbe || exclusive || !lease.active()) return exclusiveProbe;
      exclusiveProbe = (async () => {
        const current = await checked();
        if (current.state !== "in_progress" || !Number.isSafeInteger(current.totalBytes) || current.totalBytes <= 0) return;
        const url = current.finalUrl || current.url;
        const parsed = new URL(url);
        if (parsed.protocol !== "https:" || parsed.search) return;
        const reply = await api.runtime.sendNativeMessage("org.stratafetch.host", {action: "probeReplay", url, size: current.totalBytes, ...(current.mime ? {mime: current.mime} : {})});
        if (!reply?.replayable || !lease.active()) return;
        cancelWork = (async () => {
          const latest = await checked();
          if (!lease.active() || latest.state !== "in_progress") return;
          exclusive = {url, name: (latest.filename || "").split(/[\\/]/).pop(), size: latest.totalBytes};
          // Persist the recovery recipe before stopping the original request.
          await api.storage.local.set({["capture_pending_" + item.id]: record()});
          if (!lease.active()) { exclusive = undefined; return; }
          await api.downloads.cancel(item.id);
          await waitForBrowserCancellation(item);
          recordEvent("browser_stopped_before_choice", {id: item.id, state: "interrupted"});
        })();
        await cancelWork;
      })().catch(() => {}).finally(() => { exclusiveProbe = undefined; });
      return exclusiveProbe;
    };
    lease.tryExclusive();
    // Do not serialize the prompt behind pause negotiation or metadata events.
    lease.ensure().then(current => recordEvent(current.state === "complete" ? "browser_completed" : "browser_paused", current)).catch(() => {});
    const reply = await queuedCapture(async () => {
      const current = await checked();
      const settings = await api.storage.local.get({capture: true, excludedDomains: []});
      ownedStartTime = String(current.startTime || "");
      await api.storage.local.set({["capture_pending_" + item.id]: record()});
      if (!settings.capture || excluded(current.finalUrl || current.url, settings.excludedDomains)) throw new Error("Automatic capture disabled for this download");
      const payload = { action: "capture", url: current.finalUrl || current.url };
      const filename = (current.filename || item.filename || "").split(/[\\/]/).pop();
      if (filename) payload.name = filename;
      if (Number.isSafeInteger(current.totalBytes) && (current.totalBytes > 0 || (current.totalBytes === 0 && current.fileSize === 0))) payload.size = current.totalBytes;
      if (current.mime) payload.mime = current.mime;
      phase = "Opening browser bridge";
      recordEvent("confirmation_requested", current);
      return payload;
    }, async () => {
      phase = "Checking browser before starting StrataFetch";
      await lease.tryExclusive();
      const current = exclusive ? await checked() : await lease.ensure();
      if (exclusive && current.state === "in_progress") throw new Error("Browser cancellation was not confirmed; native download was not started");
      const settings = await api.storage.local.get({capture: true, excludedDomains: []});
      if (!settings.capture || excluded(current.finalUrl || current.url, settings.excludedDomains)) throw new Error("Automatic capture disabled for this download");
      recordEvent("native_commit_authorized", current);
      if (current.state === "complete") {
        if (!current.filename || current.exists === false || !Number.isSafeInteger(current.fileSize) || current.fileSize < 0) throw new Error("Completed browser file cannot be verified");
        return {action: "adoptCapture", path: current.filename, size: current.fileSize};
      }
      if (!exclusive && !current.paused) throw new Error("The browser pause could not be held after consent");
      const commit = {action: "commitCapture", url: current.finalUrl || current.url};
      const name = (current.filename || item.filename || "").split(/[\\/]/).pop();
      if (name) commit.name = name;
      if (current.mime) commit.mime = current.mime;
      if (Number.isSafeInteger(current.totalBytes) && current.totalBytes > 0) commit.size = current.totalBytes;
      return commit;
    });
    if (!reply?.ok) {
      recordEvent(reply?.declined ? "declined" : "native_rejected", item);
      if (!reply?.declined) await showResult(false, reply?.error || "Handoff failed; keeping the browser download.");
      return;
    }
    accepted = reply.download?.id;
    if (typeof accepted !== "string" || !accepted) { accepted = undefined; throw new Error("Native host did not acknowledge the download"); }
    await api.storage.local.set({["capture_pending_" + item.id]: record(accepted)});
    recordEvent("native_acknowledged", {id: item.id}, accepted);
    if (reply.adopted === true) {
      // The native manager now references this exact completed file. Erase only
      // browser history; never delete/re-download the adopted bytes.
      accepted = undefined;
      nativeOwns = true;
      paused = false;
      await api.downloads.erase({id: item.id});
      recordEvent("browser_completed_file_adopted", {id: item.id});
      await showResult(true, "Completed file moved to StrataFetch");
      return;
    }
    const latest = exclusive ? await checked() : await lease.ensure();
    if (exclusive && latest.state === "in_progress") throw new Error("The stopped browser request became active again");
    const latestIssue = exclusive ? "" : handoffIssue(latest, item);
    if (latestIssue) throw new Error(latestIssue);
    phase = "Cancelling browser copy after acknowledgment";
    if (!exclusive) await api.downloads.cancel(item.id);
    await waitForBrowserCancellation(item);
    recordEvent("browser_cancelled", {...latest, state: "interrupted", paused: false}, accepted);
    paused = false;
    accepted = undefined;
    nativeOwns = true;
    if (exclusive) await api.downloads.erase({id: item.id});
    await showResult(true, "Download moved to StrataFetch");
  } catch (error) {
    recordEvent("handoff_failed", {id: item.id}, accepted);
    await showResult(false, phase + ": " + safeIssue(error) + ". Keeping the download in your browser.");
  } finally {
    pauseLeases.delete(item.id);
    if (lease) await lease.release();
    if (cancelWork) { try { await cancelWork; } catch { /* Restore below if cancellation took effect. */ } }
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
    if (exclusive && recovered && !nativeOwns) {
      try {
        const [current] = await api.downloads.search({id: item.id});
        if (current?.state === "interrupted" && current.error === "USER_CANCELED" && sameDownload(current, item)) {
          if (eligible({...current, state: "in_progress"}, true)) {
            await restartBrowser(exclusive);
            await api.downloads.erase({id: item.id});
          } else await showResult(false, "Browser safety or privacy status changed. The stopped download was not restarted.");
          paused = false;
        }
      } catch { recovered = false; await showResult(false, "Browser restart failed. Retry the download from its original page."); }
    }
    if (paused && recovered && !nativeOwns) {
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
      await api.storage.local.remove(key); // Retired metadata-wait bookkeeping.
      continue;
    }
    if (!/^capture_pending_\d+$/.test(key)) continue;
    const id = Number(key.slice("capture_pending_".length));
    if (pending.has(id)) continue;
    let recovered = false;
    try {
      const [item] = await api.downloads.search({id});
      if (pending.has(id)) continue;
      const latest = await api.storage.local.get({[key]: null});
      if (pending.has(id) || JSON.stringify(latest[key]) !== JSON.stringify(record)) continue;
      if (record?.exclusive && item?.state === "interrupted" && item.error === "USER_CANCELED" && String(item.startTime) === record.startTime) {
        if (typeof record.nativeId === "string" && !(await nativeMessage({action: "cancel", id: record.nativeId}))?.ok) throw new Error("Native rollback is unconfirmed");
        if (eligible({...item, state: "in_progress"}, true)) {
          await restartBrowser(record.exclusive);
          await api.downloads.erase({id});
          await showResult(false, "An interrupted handoff was restarted in the browser.");
        } else await showResult(false, "Browser safety or privacy status changed. The stopped download was not restarted.");
        await api.storage.local.remove(key);
        continue;
      }
      if (item?.paused && item.state === "in_progress" && record?.startTime && String(item.startTime) === record.startTime) {
        if (typeof record.nativeId === "string") {
          try { if (!(await nativeMessage({action: "cancel", id: record.nativeId}))?.ok) throw new Error("Rollback rejected"); }
          catch { throw new Error("StrataFetch cancellation is unconfirmed"); }
        }
        recordEvent("recovery_resume_requested", item, record.nativeId);
        await api.downloads.resume(id);
        await showResult(false, "An interrupted handoff was resumed in the browser.");
      }
      recovered = true;
    } catch { await showResult(false, "Handoff recovery is incomplete. Check StrataFetch before resuming the paused browser copy."); }
    if (recovered) await api.storage.local.remove(key);
  }
}
api.runtime.onStartup.addListener(() => recoverBrowserDownloads().catch(() => {}));
const startupRecovery = recoverBrowserDownloads().catch(() => {});
