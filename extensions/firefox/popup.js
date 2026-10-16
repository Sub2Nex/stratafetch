const api = typeof browser !== "undefined" ? browser : chrome;
const status = document.getElementById("status");
const capture = document.getElementById("capture");
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
api.storage.local.get({ capture: true }).then(settings => { capture.checked = settings.capture; }).catch(() => { status.textContent = "Cannot read capture preference."; });
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
