import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "@/app/app";
import { MotionProvider } from "@/app/motion-provider";
import { ThemeProvider } from "@/app/theme-provider";
import "@fontsource-variable/inter/opsz.css";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider>
      <MotionProvider>
        <App />
      </MotionProvider>
    </ThemeProvider>
  </StrictMode>
);
