browser.runtime.onInstalled.addListener(() => {
  browser.contextMenus.create({ id: "stratafetch", title: "Download with StrataFetch", contexts: ["link"] });
});
browser.contextMenus.onClicked.addListener(async info => {
  if (info.menuItemId !== "stratafetch") return;
  try {
    const response = await browser.runtime.sendNativeMessage("org.stratafetch.host", { action: "add", url: info.linkUrl });
    await showResult(response.ok, response.ok ? "Download added to StrataFetch" : response.error);
  } catch { await showResult(false, "StrataFetch is unavailable. Start the app and register the native host."); }
});
async function showResult(ok, message) {
  await Promise.allSettled([
    browser.action.setBadgeText({ text: ok ? "+" : "!" }),
    browser.action.setBadgeBackgroundColor({ color: ok ? "#237d67" : "#a33030" }),
    browser.action.setTitle({ title: `StrataFetch: ${String(message || "Download rejected").slice(0, 240)}` }),
  ]);
}
