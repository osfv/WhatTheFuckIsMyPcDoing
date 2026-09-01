import { useState } from "react";
import { ErrorBoundary } from "./components/ErrorBoundary";
import App from "./App";

export function Root() {
  const [generation, setGeneration] = useState(0);
  return (
    <ErrorBoundary onReset={() => setGeneration((n) => n + 1)}>
      <App key={generation} />
    </ErrorBoundary>
  );
}
