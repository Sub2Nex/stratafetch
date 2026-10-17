const api = typeof browser !== "undefined" ? browser : chrome;
const locale = globalThis.StrataLocale || {t: (key, values = {}) => key.replace(/\{(\w+)\}/g, (match, name) => values[name] ?? match), error: value => value, apply() {}, setLanguage() {}};
const status = document.getElementById("status");
const capture = document.getElementById("capture");
const excluded = document.getElementById("excluded");
let statusRevision = 0, languageFromHost = false, statusKey = "Connecting…", statusValues = {}, lastIssue = "";
function render() {
  locale.apply();
  status.textContent = locale.t(statusKey, statusValues);
  const issue = document.getElementById("last-issue");
  if (issue) issue.textContent = lastIssue ? locale.t("Last handoff issue: ") + locale.error(lastIssue) : "";
}
function setStatus(key, values = {}) { statusKey = key; statusValues = values; status.textContent = locale.t(key, values); }
async function message(payload) {
  const revision = ++statusRevision;
  document.getElementById("send").disabled = true;
  try {
    const reply = await api.runtime.sendMessage(payload);
    if (reply?.language === "sv" || reply?.language === "en") { languageFromHost = true; locale.setLanguage(reply.language); render(); }
    if (revision === statusRevision) {
      if (reply?.ok) {
        if (payload.action === "ping") setStatus(reply.running === false ? "Ready · StrataFetch opens when you accept a download." : "Connected · {count} active or queued", {count: reply.active ?? 0});
        else setStatus("Added: {name}", {name: reply.download?.filename ?? locale.t("Downloads")});
      } else setStatus(reply?.declined ? "Kept in browser." : reply?.error ? locale.error(reply.error) : "StrataFetch did not respond. Open Settings → Browser integration to repair setup.");
    }
  } catch { if (revision === statusRevision) setStatus("Open StrataFetch → Settings → Browser integration to finish local setup."); }
  finally { document.getElementById("send").disabled = false; }
}
capture.onchange = async () => {
  try { await api.storage.local.set({ capture: capture.checked }); }
  catch { setStatus("Cannot save download-capture preference."); }
};
api.storage.local.get({capture: true, excludedDomains: [], lastHandoffError: null, handoffEvents: [], appLanguage: null, captureMinBytes: 0, captureExtensions: []}).then(settings => {
  if (!languageFromHost) locale.setLanguage(settings.appLanguage);
  capture.checked = settings.capture;
  excluded.value = (settings.excludedDomains || []).join("\n");
  lastIssue = settings.lastHandoffError?.message || "";
  const details = document.getElementById("handoff-events");
  if (details) details.textContent = JSON.stringify(settings.handoffEvents || [], null, 2);
  const minimum = document.getElementById("capture-minimum"), types = document.getElementById("capture-types");
  if (minimum) minimum.value = (settings.captureMinBytes || 0) / 1048576;
  if (types) types.value = (settings.captureExtensions || []).join(", ");
  render();
}).catch(() => { setStatus("Cannot read capture preference."); });
document.getElementById("save-excluded").onclick = async () => {
  ++statusRevision;
  try {
    const domains = [...new Set(excluded.value.split(/[\s,]+/).filter(Boolean).map(value => {
      const url = new URL("https://" + value.toLowerCase().replace(/^\*\./, ""));
      const host = url.hostname.replace(/\.$/, "");
      if (!host || url.username || url.password || url.port || url.pathname !== "/" || url.search || url.hash) throw new Error("Enter domain names only");
      return host;
    }))];
    if (domains.length > 100) throw new Error("Use at most 100 domain exclusions");
    await api.storage.local.set({excludedDomains: domains});
    excluded.value = domains.join("\n");
    setStatus("Exclusions saved. Matching domains and subdomains stay in the browser.");
  } catch { setStatus("Cannot save exclusions. Enter up to 100 domain names, without URLs, paths or ports."); }
};
const saveFilters = document.getElementById("save-filters");
if (saveFilters) saveFilters.onclick = async () => {
  ++statusRevision;
  try {
    const minimum = Number(document.getElementById("capture-minimum").value);
    const types = [...new Set(document.getElementById("capture-types").value.split(/[\s,]+/).filter(Boolean).map(value => value.toLowerCase().replace(/^\./, "")))];
    if (!Number.isFinite(minimum) || minimum < 0 || minimum > 1048576 || types.length > 100 || types.some(type => type.length > 32 || !/^[a-z0-9]+(?:\.[a-z0-9]+)*$/.test(type))) throw Error("Invalid filters");
    await api.storage.local.set({captureMinBytes: Math.ceil(minimum * 1048576), captureExtensions: types});
    setStatus("Capture filters saved.");
  } catch { setStatus("Invalid filters. Use 0–1048576 MiB and up to 100 file extensions such as zip, exe, iso."); }
};
document.getElementById("current").onclick = async () => {
  ++statusRevision;
  const current = document.getElementById("current");
  current.disabled = true;
  try {
    const [tab] = await api.tabs.query({ active: true, currentWindow: true });
    if (!tab?.url) throw new Error("Current tab URL is unavailable");
    document.getElementById("url").value = tab.url;
  } catch {
    document.getElementById("url").value = "";
    setStatus("Cannot read the current page URL. Paste a direct download URL instead.");
  } finally { current.disabled = false; }
};
document.getElementById("send").onclick = () => message({action: "capture", url: document.getElementById("url").value});
document.getElementById("url").onkeydown = event => {
  if (event.key === "Enter" && !document.getElementById("send").disabled) {
    event.preventDefault(); document.getElementById("send").click();
  }
};
render();
message({action: "ping"});