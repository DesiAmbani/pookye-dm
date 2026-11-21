"use strict";

const pill = document.getElementById("status");

chrome.runtime.sendMessage({ type: "status" }, (answer) => {
  if (chrome.runtime.lastError || !answer) {
    pill.textContent = "Unknown";
    return;
  }
  if (answer.connected) {
    pill.textContent = `Connected on port ${answer.port}`;
    pill.className = "pill ok";
  } else {
    pill.textContent = "Pookye DM isn't running";
  }
});
