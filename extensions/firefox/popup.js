const status = document.getElementById("status");
async function message(payload) {
  try {
    const reply = await browser.runtime.sendNativeMessage("org.stratafetch.host", payload);
    status.textContent = reply.ok ? (payload.action === "ping" ? "Connected to StrataFetch" : "Download added") : reply.error;
  } catch { status.textContent = "StrataFetch is unavailable. Start the app and register the native host."; }
}
document.getElementById("current").onclick = async () => {
  const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
  document.getElementById("url").value = tab.url || "";
};
document.getElementById("send").onclick = () => message({ action: "add", url: document.getElementById("url").value });
message({ action: "ping" });
