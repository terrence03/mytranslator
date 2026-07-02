import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Popup from "./windows/popup/Popup";
import Settings from "./windows/settings/Settings";
import "./index.css";

const label = getCurrentWindow().label;

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {label === "popup" ? <Popup /> : <Settings />}
  </React.StrictMode>,
);
