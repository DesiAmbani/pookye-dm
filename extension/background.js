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

// Cookies, referer and agent for the URL, so signed-in downloads and hotlink
// checks behave the way they would in the browser itself.
async function browserDetails(url, referer) {
  const details = { userAgent: navigator.userAgent };
  if (typeof referer === "string" && /^https?:\/\//i.test(referer)) {
    details.referer = referer;
  }
  try {
    const cookies = await chrome.cookies.getAll({ url });
    if (cookies.length > 0) {
      details.cookies = cookies.map((c) => `${c.name}=${c.value}`).join("; ");
    }
  } catch {
    // Cookie access is optional; the hand-off still works without it.
  }
  return details;
}

async function handOff(url, fileName, referer) {
  if (typeof url !== "string" || !/^https?:\/\//i.test(url)) return false;
  const port = await findApp();
  if (port === null) return false;
  try {
    const payload = { url, browser: await browserDetails(url, referer) };
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
    if (!body || body.ok !== true) return false;
    // Browser hand-offs are held until their token is confirmed.
    if (body.queued === false && body.token) {
      const confirmed = await fetch(`http://127.0.0.1:${port}/confirm`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ token: body.token }),
      });
      return confirmed.ok;
    }
    return true;
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
  const taken = await handOff(item.finalUrl || item.url, item.filename, item.referrer);
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
    void handOff(info.linkUrl, undefined, info.pageUrl);
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
