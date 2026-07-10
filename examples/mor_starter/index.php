<?php
$page_title = 'My Site';
require __DIR__ . '/includes/header.php';
?>
<main class="canvas-core" style="max-width:720px;margin:2rem auto;padding:0 1.2rem;">
  <article class="mor-post" data-edit-target="colors.bg_panel">
    <h1 data-mor-edit="site.site_title" data-field-path="site.site_title" data-edit-target="typography.heading_font_stack">My Site</h1>
    <p data-mor-edit="site.site_subtitle" data-field-path="site.site_subtitle" data-edit-target="typography.body_font_stack">A modular site, themed by design tokens.</p>
    <p>Open this folder in <strong>MorWebsite Editor</strong>. Pick a preset, tweak colors, switch the preview to <strong>Edit</strong> mode, and double-click the title.</p>
    <mor-card class="mor-card">
      <span slot="title">Web component</span>
      <p>This card themes via CSS variables from <code data-edit-target="typography.mono_font_stack">mor-theme.css</code> — no sealed hex colors.</p>
    </mor-card>
    <h2>Site Contract</h2>
    <p>DRY tokens, WET structure. See <code>docs/SITE_CONTRACT.md</code> in the editor repo.</p>
  </article>
</main>
<?php require __DIR__ . '/includes/footer.php'; ?>
