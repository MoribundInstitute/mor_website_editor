/* --- module: sidebar_toc --- */
(function () {
  var list = document.querySelector('.mor-toc-list');
  if (!list) return;
  var headings = Array.prototype.slice.call(document.querySelectorAll('main h2, main h3, .canvas-core h2, .canvas-core h3'));
  if (!headings.length) return;
  headings.forEach(function (h, i) {
    if (!h.id) h.id = 'mor-sec-' + i;
    var a = document.createElement('a');
    a.href = '#' + h.id;
    a.textContent = h.textContent;
    if (h.tagName === 'H3') a.className = 'mor-toc-h3';
    list.appendChild(a);
  });
  var links = list.querySelectorAll('a');
  var seen = new IntersectionObserver(function (entries) {
    entries.forEach(function (e) {
      if (!e.isIntersecting) return;
      links.forEach(function (l) { l.classList.remove('active'); });
      var hit = list.querySelector('a[href="#' + e.target.id + '"]');
      if (hit) hit.classList.add('active');
    });
  }, { rootMargin: '0px 0px -70% 0px' });
  headings.forEach(function (h) { seen.observe(h); });
})();
