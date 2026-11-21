"use strict";

// Ports the Pookye DM desktop app listens on, in the order it tries them.
const PORTS = [19330, 19331, 19332];
const PING_TIMEOUT_MS = 1200;

async function portOf(port) {
  try {
    const response = await fetch(`http://127.0.0.1:${port}/ping`, {
      signal: AbortSignal.timeout(PING_TIMEOUT_MS),
    });
    const body = await response.json();
    return body && body.app === "pookye" ? port : null;
  } catch {
    return null;
  }
}

async function findApp() {
  for (const port of PORTS) {
    const found = await portOf(port);
    if (found !== null) return found;
  }
  return null;
}

async function handOff(url, fileName) {
  if (typeof url !== "string" || !/^https?:\/\//i.test(url)) return false;
  const port = await findApp();
  if (port === null) return false;
  try {
    const payload = { url };
    if (typeof fileName === "string" && fileName) {
      payload.fileName = fileName.split(/[\\/]/).pop();
    }
    const response = await fetch(`http://127.0.0.1:${port}/add`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (!response.ok) return false;
    const body = await response.json();
    return body && body.ok === true;
  } catch {
    return false;
  }
}

// Downloads started by this extension itself must not be captured again.
const ownUrls = new Set();

async function capture(item, suggest) {
  if (ownUrls.delete(item.url)) {
    suggest();
    return;
  }
  if (item.byExtensionId === chrome.runtime.id) {
    suggest();
    return;
  }
  if (!/^https?:\/\//i.test(item.url ?? "")) {
    suggest();
    return;
  }
  const taken = await handOff(item.finalUrl || item.url, item.filename);
  if (!taken) {
    suggest();
    return;
  }
  chrome.downloads.cancel(item.id, () => chrome.downloads.erase({ id: item.id }));
}

chrome.downloads.onDeterminingFilename.addListener((item, suggest) => {
  void capture(item, suggest);
  return true;
});

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: "pookye-download-link",
      title: "Download with Pookye DM",
      contexts: ["link"],
      targetUrlPatterns: ["http://*/*", "https://*/*"],
    });
  });
});

chrome.contextMenus.onClicked.addListener((info) => {
  if (info.menuItemId === "pookye-download-link" && info.linkUrl) {
    void handOff(info.linkUrl);
  }
});

chrome.runtime.onMessage.addListener((message, _sender, reply) => {
  if (message && message.type === "add") {
    handOff(message.url, message.fileName).then(reply);
    return true;
  }
  if (message && message.type === "status") {
    findApp().then((port) => reply({ connected: port !== null, port }));
    return true;
  }
  return false;
});
