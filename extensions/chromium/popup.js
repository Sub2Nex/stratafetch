const status = document.getElementById("status");
async function message(payload) {
  document.getElementById("send").disabled = true;
  try {
    const reply = await chrome.runtime.sendNativeMessage("org.stratafetch.host", payload);
    status.textContent = reply.ok ? (payload.action === "ping" ? `Connected · ${reply.active ?? 0} active or queued` : `Added: ${reply.download?.filename ?? "download"} (${reply.download?.status ?? "queued"})`) : reply.error;
  } catch { status.textContent = "StrataFetch is unavailable. Start the app and register the native host."; }
  finally { document.getElementById("send").disabled = false; }
}
document.getElementById("current").onclick = async () => {
  const current = document.getElementById("current");
  current.disabled = true;
  try {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (!tab?.url) throw new Error("Current tab URL is unavailable");
    document.getElementById("url").value = tab.url;
  } catch {
    document.getElementById("url").value = "";
    status.textContent = "Cannot read the current page URL. Paste a direct download URL instead.";
  } finally { current.disabled = false; }
};
document.getElementById("send").onclick = () => message({ action: "add", url: document.getElementById("url").value });
message({ action: "ping" });
