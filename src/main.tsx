import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css"
import * as Tooltip from '@radix-ui/react-tooltip';

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Tooltip.Provider>
      <App />
    </Tooltip.Provider>
  </React.StrictMode>,
);
