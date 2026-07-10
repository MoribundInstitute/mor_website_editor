<?php
// Modular chrome — swap markup freely; keep .mor-* hooks + data-mor-edit markers.
?><!doctype html>
<html lang="en" data-theme="dark" id="top">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title><?php echo htmlspecialchars($page_title ?? 'My Site', ENT_QUOTES, 'UTF-8'); ?></title>
  <link rel="stylesheet" href="/mor-theme.css" />
  <link rel="stylesheet" href="/css/site.css" />
  <script src="/components/mor-card.js" defer></script>
  <script src="/mor-theme.js" defer></script>
</head>
<body>
<header class="main-header mor-topbar" data-edit-target="colors.bg_elevated">
  <a class="mor-brand" href="/">
    <span class="mor-brand-mark">◆</span>
    <span class="mor-brand-name" data-mor-edit="site.site_title" data-field-path="site.site_title">My Site</span>
  </a>
  <nav class="mor-nav" aria-label="Primary">
    <a class="mor-pill" href="/">Home</a>
    <a class="mor-pill" href="/about.php">About</a>
  </nav>
</header>
