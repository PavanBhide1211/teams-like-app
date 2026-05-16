import React from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "./styles.css";
import { AuthScreen } from "./features/auth/AuthScreen";
import { Shell } from "./app/Shell";
import { useAuth } from "./features/auth/useAuth";

const qc = new QueryClient({
  defaultOptions: { queries: { staleTime: 30_000, retry: 1 } },
});

function Protected({ children }: { children: React.ReactNode }) {
  const token = useAuth(s => s.accessToken);
  if (!token) return <Navigate to="/auth" replace />;
  return <>{children}</>;
}

function App() {
  return (
    <QueryClientProvider client={qc}>
      <BrowserRouter>
        <Routes>
          <Route path="/auth" element={<AuthScreen />} />
          <Route path="/*" element={<Protected><Shell /></Protected>} />
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
