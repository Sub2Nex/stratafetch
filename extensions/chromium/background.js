chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({ id: "stratafetch", title: "Download with StrataFetch", contexts: ["link"] });
});
chrome.contextMenus.onClicked.addListener(async info => {
  if (info.menuItemId !== "stratafetch") return;
  try {
    const response = await chrome.runtime.sendNativeMessage("org.stratafetch.host", { action: "add", url: info.linkUrl });
    await showResult(response.ok, response.ok ? "Download added to StrataFetch" : response.error);
  } catch { await showResult(false, "StrataFetch is unavailable. Start the app and register the native host."); }
});
async function showResult(ok, message) {
  await chrome.action.setBadgeText({ text: ok ? "+" : "!" });
  await chrome.action.setBadgeBackgroundColor({ color: ok ? "#237d67" : "#a33030" });
  await chrome.action.setTitle({ title: `StrataFetch: ${String(message || "Download rejected").slice(0, 240)}` });
}
