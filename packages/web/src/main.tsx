import { Buffer } from "buffer";
// Privy's embedded wallet signer uses Buffer.from() internally
(window as any).Buffer = Buffer;

import React from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";

import App from "./App";
import { Providers } from "./lib/providers";
import "./index.css";

// Suppress known third-party warnings from @privy-io/react-auth
const originalError = console.error;
console.error = (...args: unknown[]) => {
  if (
    typeof args[0] === "string" &&
    args[0].includes("React does not recognize the `isActive` prop")
  ) {
    return;
  }
  originalError(...args);
};

createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <BrowserRouter>
      <Providers>
        <App />
      </Providers>
    </BrowserRouter>
  </React.StrictMode>,
);
