import { Component, type CSSProperties, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  onReset: () => void;
}

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error(error, info.componentStack);
  }

  private reset = (event?: { preventDefault(): void; stopPropagation(): void }) => {
    event?.preventDefault();
    event?.stopPropagation();
    if (!this.state.error) return;
    this.setState({ error: null });
    this.props.onReset();
  };

  render() {
    if (this.state.error) {
      return (
        <div style={overlayStyle}>
          <div style={titleStyle} data-tauri-drag-region>
            <span>What The F*ck Is My PC Doing?</span>
          </div>
          <div style={bodyStyle}>
            <h1 style={{ fontSize: 22, margin: 0, letterSpacing: "-0.03em" }}>The UI hit a snag</h1>
            <p style={{ color: "#9aa1ae", marginTop: 12, maxWidth: 560, lineHeight: 1.5 }}>
              {this.state.error.message}
            </p>
            {this.state.error.stack && (
              <pre style={stackStyle}>{this.state.error.stack}</pre>
            )}
            <button
              type="button"
              onPointerDown={this.reset}
              data-tauri-drag-region="false"
              style={retryStyle}
            >
              Try again
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

const noDrag = { WebkitAppRegion: "no-drag" } as CSSProperties;

const overlayStyle: CSSProperties = {
  ...noDrag,
  position: "fixed",
  inset: 0,
  zIndex: 99999,
  background: "#0e0f12",
  color: "#eceef2",
  fontFamily: "Geist Variable, Segoe UI, sans-serif",
};

const titleStyle: CSSProperties = {
  height: 40,
  display: "flex",
  alignItems: "center",
  padding: "0 14px",
  borderBottom: "1px solid rgb(255 255 255 / 0.07)",
  color: "#9aa1ae",
  fontSize: 12,
};

const bodyStyle: CSSProperties = {
  ...noDrag,
  padding: 40,
};

const stackStyle: CSSProperties = {
  marginTop: 16,
  maxWidth: 720,
  maxHeight: 180,
  overflow: "auto",
  padding: 12,
  borderRadius: 8,
  background: "#13151a",
  color: "#9aa1ae",
  fontSize: 11,
  lineHeight: 1.45,
  whiteSpace: "pre-wrap",
};

const retryStyle: CSSProperties = {
  ...noDrag,
  marginTop: 20,
  height: 36,
  padding: "0 16px",
  border: 0,
  borderRadius: 8,
  background: "#ff6b57",
  color: "#1a0d0b",
  fontWeight: 600,
  cursor: "pointer",
  pointerEvents: "auto",
};
