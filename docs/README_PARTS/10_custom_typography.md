## Custom Fonts

Since you're theming your own site folder, you have two good options for typography.

### Option 1: Self-Host (Recommended)

Drop the font file (`.woff2`, `.ttf`) into your project and declare it in the **Custom CSS** panel:

```css
@font-face {
  font-family: 'Brand Serif';
  src: url('/fonts/BrandSerif-Regular.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

:root {
  --font-heading: 'Brand Serif', Georgia, serif;
}
```

The rule ships inside your exported `mor-theme.css`; the font file travels with your site. No third-party requests, no tracking, no CORS surprises.

### Option 2: A Privacy-Friendly CDN

If you'd rather not manage font files, use [fonts.bunny.net](https://fonts.bunny.net) — the same catalog as Google Fonts, minus the tracking pixels and IP logging.

1. Pick your font family at [fonts.bunny.net](https://fonts.bunny.net).
2. Copy the generated `@import` rule.
3. Paste it into the **Custom CSS** panel.

```css
@import url('https://fonts.bunny.net/css?family=inter:400,700');

:root {
  --font-body: 'Inter', system-ui, sans-serif;
}
```

**A note on external hosts:** if you load fonts from a third-party server, strict CORS headers on that host can make the font silently fail and fall back to a default. Self-hosting avoids the problem entirely.

Custom font rules pass through the internal normalization pipeline (`resolve_font_stack()`) before export — see [DECISIONS.md](DECISIONS.md).
