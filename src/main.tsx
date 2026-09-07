import React from "react";
import ReactDOM from "react-dom/client";

import App from "@/App";
import RecordingBar from "@/windows/RecordingBar";
import "@/styles/main.css";

const isRecordingBar =
  window.location.hash === "#recording-bar" ||
  new URLSearchParams(window.location.search).get("view") === "recording-bar";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{isRecordingBar ? <RecordingBar /> : <App />}</React.StrictMode>,
);
