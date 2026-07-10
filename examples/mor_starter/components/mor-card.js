/**
 * <mor-card> — minimal web component that themes from CSS variables only.
 * Site Contract: no hard-coded hex; editor restyles via --card-* / --bg-panel.
 */
class MorCard extends HTMLElement {
  constructor() {
    super();
    const root = this.attachShadow({ mode: "open" });
    root.innerHTML = `
      <style>
        :host {
          display: block;
          margin: 1.2rem 0;
          padding: 1rem 1.15rem;
          border-radius: 10px;
          border: 1px solid var(--border-color, #333);
          background: var(--card-bg, var(--bg-panel, #151d29));
          color: var(--card-fg, var(--fg-base, #ddd));
          font-family: var(--font-body, system-ui, sans-serif);
        }
        .title {
          display: block;
          margin: 0 0 0.45rem;
          font-family: var(--font-heading, inherit);
          font-weight: 600;
          color: var(--card-accent, var(--accent, #7aa2f7));
        }
        ::slotted(p) { margin: 0; line-height: 1.55; opacity: 0.92; }
      </style>
      <span class="title"><slot name="title">Card</slot></span>
      <slot></slot>
    `;
  }
}
if (!customElements.get("mor-card")) {
  customElements.define("mor-card", MorCard);
}
