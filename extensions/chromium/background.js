chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({ id: "stratafetch", title: "Download with StrataFetch", contexts: ["link"] });
});
chrome.contextMenus.onClicked.addListener(async info => {
  if (info.menuItemId !== "stratafetch") return;
  try {
    const response = await chrome.runtime.sendNativeMessage("org.stratafetch.host", { action: "add", url: info.linkUrl });
    if (!response.ok) console.error(response.error);
  } catch (error) { console.error("Start StrataFetch and install the native host.", error); }
});
