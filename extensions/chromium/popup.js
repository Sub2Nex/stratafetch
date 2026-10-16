const api = typeof browser !== "undefined" ? browser : chrome;
const status = document.getElementById("status");
const capture = document.getElementById("capture");
const excluded = document.getElementById("excluded");
let statusRevision = 0;
async function message(payload) {
  const revision = ++statusRevision;
  document.getElementById("send").disabled = true;
  try {
    const reply = await api.runtime.sendMessage(payload);
    if (revision === statusRevision) status.textContent = reply?.ok ? (payload.action === "ping" ? reply.running === false ? "Ready · StrataFetch opens when you accept a download." : `Connected · ${reply.active ?? 0} active or queued` : `Added: ${reply.download?.filename ?? "download"}`) : reply?.declined ? "Kept in browser." : reply?.error || "StrataFetch did not respond. Open Settings → Browser integration to repair setup.";
  } catch { if (revision === statusRevision) status.textContent = "Open StrataFetch → Settings → Browser integration to finish local setup."; }
  finally { document.getElementById("send").disabled = false; }
}
capture.onchange = async () => {
  try { await api.storage.local.set({ capture: capture.checked }); }
  catch { status.textContent = "Cannot save download-capture preference."; }
};
api.storage.local.get({ capture: true, excludedDomains: [], lastHandoffError: null }).then(settings => {
  capture.checked = settings.capture;
  excluded.value = (settings.excludedDomains || []).join("\n");
  const issue = document.getElementById("last-issue");
  if (issue && settings.lastHandoffError?.message) issue.textContent = "Last handoff issue: " + settings.lastHandoffError.message;
}).catch(() => { status.textContent = "Cannot read capture preference."; });
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
    status.textContent = "Exclusions saved. Matching domains and subdomains stay in the browser.";
  } catch { status.textContent = "Cannot save exclusions. Enter up to 100 domain names, without URLs, paths or ports."; }
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
    status.textContent = "Cannot read the current page URL. Paste a direct download URL instead.";
  } finally { current.disabled = false; }
};
document.getElementById("send").onclick = () => message({ action: "capture", url: document.getElementById("url").value });
message({ action: "ping" });
