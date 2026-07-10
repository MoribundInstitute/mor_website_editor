<?php
$page_title = 'About · My Site';
require __DIR__ . '/includes/header.php';
?>
<main class="canvas-core" style="max-width:720px;margin:2rem auto;padding:0 1.2rem;">
  <article class="mor-post">
    <h1 data-edit-target="typography.heading_font_stack">About</h1>
    <p data-edit-target="typography.body_font_stack">A second page on purpose — some HTML duplication is fine when both pages share the same <code>.mor-*</code> hooks and link the same theme file.</p>
    <p data-mor-edit="site.site_subtitle" data-field-path="site.site_subtitle">A modular site, themed by design tokens.</p>
  </article>
</main>
<?php require __DIR__ . '/includes/footer.php'; ?>
