use dioxus::prelude::*;

use crate::app::edit_context::{EditContext, SelectionInfo};
use crate::app::state::ContextMenuPayload;
use crate::ui::workspace::layout::{clamp_preview_width, PreviewViewport};

const SCALER_JS: &str = r#"
(function() {
    function initScaler() {
        const wrapper = document.querySelector('.preview-scale-wrapper');
        const frame = document.getElementById('mor-preview-device-frame');
        if (!wrapper || !frame) return;

        let lastScale = null;
        function applyScale(scale) {
            if (scale === lastScale) return;
            lastScale = scale;
            wrapper.style.setProperty('--preview-scale', scale);
        }

        function scaleFrame() {
            if (frame.classList.contains('preview-device-frame-fit')) {
                applyScale(1);
                return;
            }

            const targetWidth = parseFloat(frame.style.width);
            if (!targetWidth) return;

            const availableWidth = wrapper.clientWidth - 48;

            if (targetWidth > availableWidth && availableWidth > 0) {
                applyScale(availableWidth / targetWidth);
            } else {
                applyScale(1);
            }
        }

        let rafPending = false;
        function scheduleScale() {
            if (rafPending) return;
            rafPending = true;
            requestAnimationFrame(function () {
                rafPending = false;
                scaleFrame();
            });
        }

        if (window.__morScalerObs) {
            window.__morScalerObs.disconnect();
        }
        window.__morScalerObs = new ResizeObserver(scheduleScale);
        window.__morScalerObs.observe(wrapper);
        scaleFrame();
    }

    initScaler();
    setTimeout(initScaler, 50);
})();
"#;

#[component]
pub fn PreviewCanvas(
    preview_viewport: Signal<PreviewViewport>,
    preview_width: Signal<u32>,
    preview_html: String,
    #[props(default)] xray_active: Option<Signal<bool>>,
    /// Edit mode: outlines + selection ride `xray_active`; mutating gestures
    /// (text dblclick, widget drag, SVG drop, icon shift-click) need this too.
    #[props(default)] edit_active: Option<Signal<bool>>,
    #[props(default)] on_navigate: Option<EventHandler<String>>,
    #[props(default)] on_select: Option<EventHandler<String>>,
    /// Click on a node with no marker: DOM facts for Inspector classification.
    #[props(default)] on_select_dom: Option<EventHandler<crate::app::edit_context::DomSelectFacts>>,
    /// Last analyzed selection, rendered as a compact chip (not an X-Ray wash).
    #[props(default)] active_selection: Option<Signal<Option<SelectionInfo>>>,
    #[props(default)] on_icon_edit: Option<EventHandler<String>>,
    #[props(default)] on_icon_context_menu: Option<EventHandler<ContextMenuPayload>>,
    #[props(default)] on_update_value: Option<EventHandler<(String, String)>>,
    /// Unbound page-content edit: (old_text, new_text) → unique replace in page file.
    #[props(default)] on_page_text_edit: Option<EventHandler<(String, String)>>,
    #[props(default)] on_move_widget: Option<EventHandler<(String, String)>>,
    #[props(default)] on_toggle_dark_mode: Option<EventHandler<()>>,
    #[props(default)] on_drop_svg: Option<EventHandler<(String, String)>>,
    /// Transient status (drop hints, etc.) — parent usually forwards to the status bar.
    #[props(default)] on_status: Option<EventHandler<String>>,
    /// Active theme preset label shown on the ruler (how the site is styled).
    #[props(default)] preset_label: Option<String>,
) -> Element {
    if let Some(xray_signal) = xray_active {
        use_effect(move || {
            let active = xray_signal();
            spawn(async move {
                let js = format!(
                    "window.__morXrayActive={active};if(window.morApplyXray)window.morApplyXray({active});"
                );
                let _ = dioxus::document::eval(&js);
            });
        });
    }

    // The gate is a window global that outlives workspace switches, so a
    // canvas without a mode switch (static page editor, workbenches) must
    // reset it or it inherits the last workspace's Browse/Inspect lock.
    if let Some(edit_signal) = edit_active {
        use_effect(move || {
            let active = edit_signal();
            spawn(async move {
                let _ = dioxus::document::eval(&format!("window.__morEditActive={active};"));
            });
        });
    } else {
        use_effect(move || {
            spawn(async move {
                let _ = dioxus::document::eval("window.__morEditActive=true;");
            });
        });
    }

    let mut preview_width = preview_width;
    let mut preview_viewport = preview_viewport;
    // Drag-to-resize state: whether a drag is active, and (start cursor x, start width).
    let mut dragging = use_signal(|| false);
    let mut drag_start = use_signal(|| (0.0_f64, 0u32));

    let xray_on = xray_active.map(|s| s()).unwrap_or(false);
    let edit_on = edit_active.map(|s| s()).unwrap_or(false);
    let current_viewport = preview_viewport();
    let viewport_label = current_viewport.label();
    let look = if edit_on {
        "Edit"
    } else if xray_on {
        "Select"
    } else {
        "View · as published"
    };
    let preset_bit = preset_label
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| format!(" · preset {s}"))
        .unwrap_or_default();
    let viewport_meta = if current_viewport == PreviewViewport::Fit {
        format!("Fit · available width · {look}{preset_bit}")
    } else {
        format!(
            "{} · {}px · {look}{preset_bit}",
            viewport_label,
            preview_width()
        )
    };

    let device_class = if current_viewport == PreviewViewport::Fit {
        "preview-device-frame preview-device-frame-fit"
    } else if dragging() {
        // Kill the width transition mid-drag so the frame tracks the cursor 1:1.
        "preview-device-frame preview-device-frame-resizing"
    } else {
        "preview-device-frame"
    };

    let device_style = if current_viewport == PreviewViewport::Fit {
        String::new()
    } else {
        format!("width: {}px;", preview_width())
    };

    rsx! {
        div {
            class: "preview-canvas",

            // Compact selection chip — no X-Ray legend overlay on the page.
            if xray_on {
                if let Some(sel) = active_selection.and_then(|s| s()) {
                    div {
                        class: if sel.context == EditContext::CodeOnly { "preview-selection-chip preview-selection-chip-unbound" } else { "preview-selection-chip" },
                        title: "Selected in the preview — open Inspector (Alt+X) for fields",
                        "{sel.label}"
                    }
                }
            }

            div {
                class: "preview-ruler",
                span {
                    class: "preview-ruler-label",
                    "{viewport_meta}"
                }
                div { class: "preview-ruler-line" }
            }

            div {
                class: "preview-scale-wrapper",

                div {
                    class: "{device_class}",
                    id: "mor-preview-device-frame",
                    style: "{device_style}",

                    pre {
                        id: "mor-preview-html-source",
                        style: "display: none;",
                        "{preview_html}"
                    }

                    iframe {
                        key: "mor-preview-frame-stable",
                        id: "mor-preview-frame",
                        class: "preview-iframe",
                        src: "about:blank",
                        onmounted: move |_| {
                            spawn(async move {
                                let mut eval = dioxus::document::eval(
                                    r#"
                                    (function() {
                                        const SID = "mor-preview-html-source", FID = "mor-preview-frame";

                                        // Full document rewrite. Uses document.write (not innerHTML) so that
                                        // <script> tags (plugin JS) actually execute on (re)load.
                                        function reload(doc, html) {
                                            doc.open(); doc.write(html); doc.close();
                                            // document.write() wipes the document's event listeners, but the
                                            // _inst guard set on the document object survives it — so without
                                            // clearing it, setup() bails and never re-attaches the click /
                                            // edit / drag handlers. That left the preview "stuck" after the
                                            // first dark/light toggle (which reloads): the toggle button no
                                            // longer responded. Force a fresh setup on every reload.
                                            doc._inst = false;
                                            doc._morBodySrc = undefined;
                                            setTimeout(() => setup(doc), 50);
                                        }

                                        // Append ?mor_r=<ts> to same-origin / relative CSS+JS so a hard
                                        // refresh actually re-fetches stylesheets the browser may have
                                        // cached from the local preview server.
                                        function cacheBustLocalAssets(html) {
                                            var t = Date.now();
                                            return html.replace(
                                                /\b(href|src)=(["'])([^"']+)\2/gi,
                                                function (_m, attr, q, url) {
                                                    if (/^(data:|blob:|mailto:|javascript:)/i.test(url)) return _m;
                                                    if (/^https?:\/\//i.test(url) && url.indexOf('127.0.0.1') < 0 && url.indexOf('localhost') < 0) {
                                                        return _m;
                                                    }
                                                    if (!/\.(css|js)(\?|#|$)/i.test(url)) return _m;
                                                    var bare = url.split('#')[0];
                                                    var hash = url.indexOf('#') >= 0 ? url.slice(url.indexOf('#')) : '';
                                                    var sep = bare.indexOf('?') >= 0 ? '&' : '?';
                                                    // Drop a prior mor_r so repeated hard refreshes stay clean.
                                                    bare = bare.replace(/([?&])mor_r=\d+&?/g, '$1').replace(/[?&]$/, '');
                                                    sep = bare.indexOf('?') >= 0 ? '&' : '?';
                                                    return attr + '=' + q + bare + sep + 'mor_r=' + t + hash + q;
                                                }
                                            );
                                        }

                                        // Ctrl+Shift+R / View → Hard Refresh Preview.
                                        // Clears the morph cache and force-rewrites the iframe document.
                                        window.__morHardRefreshPreview = function () {
                                            var src = document.getElementById(SID);
                                            var frm = document.getElementById(FID);
                                            if (src) src._last = null;
                                            if (!src || !frm) return;
                                            var html = src.textContent || '';
                                            if (!html.trim()) return;
                                            var doc = frm.contentDocument || (frm.contentWindow && frm.contentWindow.document);
                                            if (!doc) return;
                                            src._last = html; // avoid a double-write when React/Dioxus re-renders
                                            reload(doc, cacheBustLocalAssets(html));
                                        };

                                        // Stable identity for an editable node, used to detect structural
                                        // reorders (a widget moving between regions) vs pure content edits.
                                        function keyOf(el) {
                                            return el.getAttribute('data-block-id')
                                                || el.getAttribute('data-field-path')
                                                || el.getAttribute('data-edit-target')
                                                || '';
                                        }

                                        function write(src, frm) {
                                            const html = src.textContent || "";
                                            if (!html.trim() || src._last === html) return;
                                            src._last = html;
                                            const doc = frm.contentDocument || frm.contentWindow.document;
                                            if (!doc) return;

                                            const nDoc = new DOMParser().parseFromString(html, 'text/html');
                                            // Source-body identity, tracked on the doc object (which survives
                                            // document.write). The live body diverges at runtime (site JS, web
                                            // components), so compare source-vs-source, never live-vs-source.
                                            const nBody = nDoc.body ? nDoc.body.innerHTML : "";
                                            const bodyChanged = doc._morBodySrc !== nBody;
                                            doc._morBodySrc = nBody;

                                            // First paint (blank iframe) or empty body.
                                            if (!doc.body || !doc.body.innerHTML.trim()) {
                                                reload(doc, html);
                                                return;
                                            }
                                            const oCss = doc.getElementById('mor-true-css'), nCss = nDoc.getElementById('mor-true-css');
                                            if (oCss && nCss && oCss.textContent !== nCss.textContent) oCss.textContent = nCss.textContent;
                                            if (doc.body.style.cssText !== nDoc.body.style.cssText) doc.body.style.cssText = nDoc.body.style.cssText;
                                            if (doc.documentElement.style.cssText !== nDoc.documentElement.style.cssText) doc.documentElement.style.cssText = nDoc.documentElement.style.cssText;
                                            if (doc.documentElement.className !== nDoc.documentElement.className) doc.documentElement.className = nDoc.documentElement.className;
                                            if (doc.documentElement.getAttribute('data-theme') !== nDoc.documentElement.getAttribute('data-theme')) doc.documentElement.setAttribute('data-theme', nDoc.documentElement.getAttribute('data-theme') || "");
                                            if (doc.body.className !== nDoc.body.className) doc.body.className = nDoc.body.className;

                                            doc.querySelectorAll('link[href*="fonts.googleapis"], link[href*="fonts.gstatic"], style').forEach(el => {
                                                if (el.id !== 'mor-true-css' && !el.textContent.includes('[data-field-path]')) el.remove();
                                            });
                                            nDoc.querySelectorAll('link[href*="fonts.googleapis"], link[href*="fonts.gstatic"], style').forEach(el => { if (el.id !== 'mor-true-css') doc.head.appendChild(el.cloneNode(true)); });

                                            const oT = doc.querySelectorAll('[data-field-path], [data-block-id], [data-edit-target]');
                                            const nT = nDoc.querySelectorAll('[data-field-path], [data-block-id], [data-edit-target]');

                                            // Real-website pages carry no editable markers, so the keyed
                                            // morph below can't see their content. When the *source* body
                                            // changed (page switch, refetch after a file edit), reload;
                                            // otherwise only css/style patches above were needed and the
                                            // iframe keeps its scroll and JS state.
                                            if (oT.length === 0 && nT.length === 0) {
                                                if (bodyChanged) reload(doc, html);
                                                return;
                                            }

                                            // Match editable nodes by key AND verify identical order. If the
                                            // set or ordering changed (e.g. a widget moved regions), an in-place
                                            // innerHTML patch would scramble content across slots, so reload.
                                            let sameShape = (oT.length === nT.length);
                                            if (sameShape) {
                                                for (let i = 0; i < oT.length; i++) {
                                                    if (keyOf(oT[i]) !== keyOf(nT[i])) { sameShape = false; break; }
                                                }
                                            }
                                            if (!sameShape) { reload(doc, html); return; }

                                            // Same nodes, same order: only inner content can differ, so index
                                            // pairing is now provably safe. Skip a field the user is actively
                                            // editing so a background re-render doesn't wipe the caret / text.
                                            oT.forEach((el, i) => {
                                                if (el.isContentEditable && doc.activeElement === el) return;
                                                if (el.innerHTML !== nT[i].innerHTML) el.innerHTML = nT[i].innerHTML;
                                                if (el.hasAttribute('data-block-id')) el.draggable = true;
                                            });
                                        }
                                        // Light edit chrome only — soft hover, no region washes.
                                        const XRAY_CSS = `
html.mor-xray-on .mor-xray-hover{outline:2px solid rgba(40,149,240,.88)!important;outline-offset:2px;cursor:pointer;box-shadow:0 0 0 3px rgba(40,149,240,.14);transition:outline-color .12s ease,box-shadow .12s ease}
html.mor-xray-on [data-block-id]{cursor:grab;transition:outline-color .12s ease,box-shadow .12s ease}
html.mor-xray-on [data-block-id]:hover{outline:1.5px dashed rgba(34,197,94,.75);outline-offset:2px}
html.mor-xray-on [data-field-path],html.mor-xray-on [data-mor-edit]{cursor:text;transition:outline-color .12s ease,box-shadow .12s ease}
html.mor-xray-on [data-field-path]:hover,html.mor-xray-on [data-mor-edit]:hover{outline:1.5px dashed rgba(40,149,240,.8);outline-offset:2px;box-shadow:0 0 0 3px rgba(40,149,240,.1)}
html.mor-xray-on a:hover,html.mor-xray-on button:hover,html.mor-xray-on .btn-primary:hover,html.mor-xray-on .btn:hover{outline:1.5px solid rgba(40,149,240,.55);outline-offset:2px}
html.mor-xray-on img:hover{outline:1.5px solid rgba(40,149,240,.55);outline-offset:2px}
/* Selection: thin outline + floating label only (no background tint) */
html.mor-xray-on .mor-canvas-selected{outline:2px solid #2895f0!important;outline-offset:2px!important;position:relative!important;z-index:50;background:transparent!important;box-shadow:0 0 0 3px rgba(40,149,240,.18)!important}
html.mor-xray-on .mor-canvas-selected::before{content:attr(data-mor-sel-label);position:absolute;top:-22px;left:-2px;z-index:100050;padding:3px 7px;border-radius:2px 2px 0 0;font:600 11px/1.2 system-ui,-apple-system,sans-serif;letter-spacing:.02em;background:#2895f0;color:#fff;pointer-events:none;white-space:nowrap;box-shadow:0 1px 3px rgba(0,0,0,.25)}
html.mor-xray-on .mor-canvas-editing{outline:2px solid #146ef5!important;outline-offset:2px!important;caret-color:#146ef5;min-width:1ch;background:transparent!important;box-shadow:0 0 0 3px rgba(20,110,245,.2)!important}
html.mor-xray-on .mor-canvas-editing::before{content:attr(data-mor-sel-label) " · editing";background:#146ef5}
/* Rich-text floating bar — readable groups, clear labels */
#mor-rte-bar{position:fixed;z-index:2147483000;display:flex;flex-direction:column;gap:0;min-width:min(420px,92vw);max-width:min(560px,96vw);padding:0;border-radius:12px;background:#fff;border:1px solid #dadce0;box-shadow:0 4px 16px rgba(32,33,36,.18),0 1px 3px rgba(60,64,67,.12);font:13px/1.3 system-ui,-apple-system,Segoe UI,sans-serif;color:#202124;user-select:none}
#mor-rte-bar .mor-rte-top{display:flex;flex-wrap:wrap;align-items:center;gap:4px;padding:8px 10px 6px}
#mor-rte-bar .mor-rte-hint{padding:0 12px 8px;font:11px/1.35 system-ui,sans-serif;color:#5f6368;border-top:1px solid #f1f3f4}
#mor-rte-bar .mor-rte-group{display:inline-flex;align-items:center;gap:2px}
#mor-rte-bar button{appearance:none;border:1px solid transparent;background:transparent;color:#3c4043;min-width:32px;height:32px;padding:0 8px;border-radius:8px;cursor:pointer;font:600 12px/1 system-ui,sans-serif;display:inline-flex;align-items:center;justify-content:center;gap:4px}
#mor-rte-bar button:hover{background:#f1f3f4;border-color:#e8eaed}
#mor-rte-bar button:focus-visible{outline:2px solid #1a73e8;outline-offset:1px}
#mor-rte-bar button.is-on{background:#e8f0fe;color:#1967d2;border-color:#d2e3fc}
#mor-rte-bar button.mor-rte-primary{background:#1a73e8;color:#fff;border-color:#1a73e8;font-weight:600}
#mor-rte-bar button.mor-rte-primary:hover{background:#1765cc;border-color:#1765cc}
#mor-rte-bar button.mor-rte-quiet{color:#5f6368;font-weight:500}
#mor-rte-bar .mor-rte-sep{width:1px;height:22px;background:#dadce0;margin:0 4px;flex-shrink:0}
#mor-rte-bar select{height:32px;border:1px solid #dadce0;border-radius:8px;background:#fff;font:12px system-ui;color:#202124;max-width:128px;padding:0 8px;cursor:pointer}
#mor-rte-bar select:hover{border-color:#bdc1c6}
#mor-rte-bar .mor-rte-link-row{display:none;width:100%;gap:8px;padding:8px 10px 10px;align-items:center;flex-wrap:wrap;border-top:1px solid #f1f3f4;background:#f8f9fa;border-radius:0 0 12px 12px;box-sizing:border-box}
#mor-rte-bar.mor-rte-link-open .mor-rte-link-row{display:flex}
#mor-rte-bar .mor-rte-link-row input[type=url],#mor-rte-bar .mor-rte-link-row input[type=text]{flex:1;min-width:140px;height:32px;border:1px solid #dadce0;border-radius:8px;padding:0 10px;font:13px system-ui;background:#fff;box-sizing:border-box}
#mor-rte-bar .mor-rte-link-row input:focus{border-color:#1a73e8;outline:none;box-shadow:0 0 0 2px rgba(26,115,232,.2)}
#mor-rte-bar .mor-rte-link-label{display:flex;flex-direction:column;gap:3px;flex:1;min-width:0;font:11px/1.2 system-ui;color:#5f6368;font-weight:600}
#mor-rte-bar .mor-rte-link-check{display:flex;align-items:center;gap:6px;font:12px system-ui;color:#3c4043;white-space:nowrap;cursor:pointer;font-weight:500}
#mor-rte-bar .mor-rte-label{font:10px/1 system-ui;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:#80868b;margin:0 2px 0 4px}
.mor-canvas-editing{background:rgba(26,115,232,.04)!important;border-radius:2px}
.mor-canvas-editing a{text-decoration:underline;color:#1967d2}
.mor-drop-hover{outline:2px dashed #146ef5!important;outline-offset:3px;background:rgba(20,110,245,.06)!important}
.mor-insert-row,.mor-insert-card{min-height:2rem}`

                                        function widgetRegion(el) {
                                            if (el.closest('.panel-left')) return 'Left';
                                            if (el.closest('.panel-right')) return 'Right';
                                            if (el.closest('.canvas-core')) return 'Main';
                                            return 'Layout';
                                        }

                                        function widgetType(el) {
                                            return [...el.classList].filter(c => c !== 'widget').join(' ') || 'Widget';
                                        }

                                        function clearXrayAnnotations(doc) {
                                            doc.querySelectorAll('[data-xray-widget],[data-xray-hint]').forEach(el => {
                                                el.removeAttribute('data-xray-widget');
                                                el.removeAttribute('data-xray-hint');
                                            });
                                        }

                                        function annotateXray(doc) {
                                            clearXrayAnnotations(doc);
                                            doc.querySelectorAll('[data-block-id]').forEach(el => {
                                                const id = el.getAttribute('data-block-id') || 'widget';
                                                el.setAttribute('data-xray-widget', widgetRegion(el) + ' · ' + id + ' (' + widgetType(el) + ')');
                                            });
                                            doc.querySelectorAll('[data-field-path],[data-mor-edit]').forEach(el => {
                                                const path = el.getAttribute('data-field-path') || el.getAttribute('data-mor-edit');
                                                el.setAttribute('data-xray-hint', 'Dbl-click text · ' + path);
                                            });
                                            doc.querySelectorAll('[data-edit-target^="icons."]').forEach(el => {
                                                el.setAttribute('data-xray-hint', 'Shift-click icon · ' + el.getAttribute('data-edit-target'));
                                            });
                                            doc.querySelectorAll('[data-edit-target]:not([data-field-path]):not([data-mor-edit]):not([data-block-id])').forEach(el => {
                                                const target = el.getAttribute('data-edit-target') || '';
                                                if (target.startsWith('icons.')) return;
                                                el.setAttribute('data-xray-hint', 'Theme token · ' + target);
                                            });
                                        }

                                        function ensureXrayStyle(doc) {
                                            let style = doc.getElementById('mor-xray-style');
                                            if (!style) {
                                                style = doc.createElement('style');
                                                style.id = 'mor-xray-style';
                                                style.textContent = XRAY_CSS;
                                                doc.head.appendChild(style);
                                            }
                                        }

                                        function applyXray(doc, active) {
                                            ensureXrayStyle(doc);
                                            doc.documentElement.classList.toggle('mor-xray-on', !!active);
                                            if (active) annotateXray(doc);
                                            else {
                                                clearXrayAnnotations(doc);
                                                doc.querySelectorAll('.mor-xray-hover').forEach(el => el.classList.remove('mor-xray-hover'));
                                            }
                                        }

                                        window.morApplyXray = function(active) {
                                            window.__morXrayActive = !!active;
                                            const frm = document.getElementById(FID);
                                            const doc = frm && (frm.contentDocument || frm.contentWindow.document);
                                            if (doc && doc.body) applyXray(doc, active);
                                        };

                                        function setup(doc) {
                                            if (doc._inst) return; doc._inst = true;
                                            // Site Contract: data-mor-edit is the public name;
                                            // normalize to data-field-path the bridge already uses.
                                            doc.querySelectorAll('[data-mor-edit]').forEach(el => {
                                                if (!el.getAttribute('data-field-path')) {
                                                    el.setAttribute('data-field-path', el.getAttribute('data-mor-edit'));
                                                }
                                            });
                                            const s = doc.createElement('style');
                                            s.id = 'mor-edit-style';
                                            // Soft edit affordances — no region paints; site stays looking published.
                                            s.textContent = `html.mor-xray-on [data-field-path]:hover,html.mor-xray-on [data-mor-edit]:hover{outline:1.5px dashed rgba(40,149,240,.7);outline-offset:2px;cursor:text} [data-block-id],[data-mor-block]{cursor:grab;position:relative} .dragging{opacity:0.5} .drag-over,.mor-drop-hover{outline:2px dashed #146ef5;outline-offset:2px} .mor-canvas-editing{cursor:text!important}`;
                                            doc.head.appendChild(s);
                                            doc.querySelectorAll('[data-block-id]').forEach(el => el.draggable = true);

                                            const TEXT_EDIT = 'h1,h2,h3,h4,h5,h6,p,span,a,li,label,button,td,th,figcaption,blockquote,strong,em,small,div';
                                            let contentDragEl = null;

                                            function contentHosts(doc) {
                                                const list = doc.querySelectorAll('.canvas-core, main, article, .content, #content');
                                                return list.length ? Array.from(list) : [doc.body];
                                            }
                                            function markContentBlocks(doc) {
                                                contentHosts(doc).forEach(host => {
                                                    Array.from(host.children || []).forEach(el => {
                                                        if (!el || el.nodeType !== 1) return;
                                                        if (/^(SCRIPT|STYLE|LINK|NOSCRIPT|META)$/i.test(el.tagName)) return;
                                                        if (el.id === 'mor-rte-bar') return;
                                                        el.setAttribute('data-mor-block', '1');
                                                        if (!el.getAttribute('data-block-id')) el.draggable = true;
                                                    });
                                                });
                                            }
                                            markContentBlocks(doc);

                                            // ── Rich text toolbar (readable groups + Done) ────────
                                            function hideRichToolbar() {
                                                const b = doc.getElementById('mor-rte-bar');
                                                if (b) {
                                                    if (b._cleanup) try { b._cleanup(); } catch (_) {}
                                                    b.remove();
                                                }
                                            }
                                            function placeRichToolbar(bar, el) {
                                                const r = el.getBoundingClientRect();
                                                const vw = doc.defaultView.innerWidth || 800;
                                                const vh = doc.defaultView.innerHeight || 600;
                                                const bw = bar.offsetWidth || 440;
                                                const bh = bar.offsetHeight || 88;
                                                let top = r.top - bh - 10;
                                                if (top < 8) top = Math.min(r.bottom + 10, vh - bh - 8);
                                                let left = Math.max(8, Math.min(r.left + (r.width / 2) - (bw / 2), vw - bw - 8));
                                                bar.style.top = Math.max(8, top) + 'px';
                                                bar.style.left = left + 'px';
                                            }
                                            function finishRichEdit(el, save) {
                                                if (!el || el.contentEditable !== 'true') return;
                                                const bar = doc.getElementById('mor-rte-bar');
                                                if (bar && bar._cleanup) try { bar._cleanup(); } catch (_) {}
                                                hideRichToolbar();
                                                window.__morRichActive = null;
                                                const beforeText = el.getAttribute('data-mor-edit-before') || '';
                                                const beforeHtml = el.getAttribute('data-mor-edit-before-html') || '';
                                                if (!save) {
                                                    if (beforeHtml != null) el.innerHTML = beforeHtml;
                                                    else if (beforeText != null) el.innerText = beforeText;
                                                }
                                                el.contentEditable = 'false';
                                                el.classList.remove('mor-canvas-editing');
                                                el.removeAttribute('data-mor-edit-before');
                                                el.removeAttribute('data-mor-edit-before-html');
                                                if (!save) return;
                                                const afterText = el.innerText;
                                                const afterHtml = el.innerHTML;
                                                const path = el.getAttribute('data-field-path') || el.getAttribute('data-mor-edit');
                                                if (path) {
                                                    dioxus.send({action: "UPDATE_VALUE", target: path, value: afterText});
                                                } else {
                                                    const useHtml = beforeHtml !== afterHtml
                                                        && (beforeHtml.includes('<') || afterHtml.includes('<'));
                                                    const oldT = useHtml ? beforeHtml : beforeText;
                                                    const newT = useHtml ? afterHtml : afterText;
                                                    if (oldT !== newT) {
                                                        dioxus.send({
                                                            action: "PAGE_TEXT_EDIT",
                                                            old_text: oldT,
                                                            new_text: newT,
                                                            tag: (el.tagName || '').toLowerCase(),
                                                            rich: useHtml
                                                        });
                                                    }
                                                }
                                            }
                                            function showRichToolbar(el) {
                                                hideRichToolbar();
                                                const bar = doc.createElement('div');
                                                bar.id = 'mor-rte-bar';
                                                bar.setAttribute('contenteditable', 'false');
                                                bar.setAttribute('role', 'toolbar');
                                                bar.setAttribute('aria-label', 'Text formatting');
                                                bar.innerHTML = `
                                                  <div class="mor-rte-top">
                                                    <span class="mor-rte-label">Style</span>
                                                    <select data-block title="Paragraph style" aria-label="Paragraph style">
                                                      <option value="p">Paragraph</option>
                                                      <option value="h1">Heading 1</option>
                                                      <option value="h2">Heading 2</option>
                                                      <option value="h3">Heading 3</option>
                                                      <option value="blockquote">Quote</option>
                                                    </select>
                                                    <span class="mor-rte-sep"></span>
                                                    <div class="mor-rte-group" role="group" aria-label="Emphasis">
                                                      <button type="button" data-cmd="bold" title="Bold (Ctrl+B)" aria-label="Bold"><b>B</b></button>
                                                      <button type="button" data-cmd="italic" title="Italic (Ctrl+I)" aria-label="Italic"><i>I</i></button>
                                                      <button type="button" data-cmd="underline" title="Underline (Ctrl+U)" aria-label="Underline"><u>U</u></button>
                                                      <button type="button" data-cmd="strikeThrough" title="Strikethrough" aria-label="Strikethrough"><s>S</s></button>
                                                    </div>
                                                    <span class="mor-rte-sep"></span>
                                                    <div class="mor-rte-group" role="group" aria-label="Lists">
                                                      <button type="button" data-cmd="insertUnorderedList" title="Bullet list" aria-label="Bullet list">• List</button>
                                                      <button type="button" data-cmd="insertOrderedList" title="Numbered list" aria-label="Numbered list">1. List</button>
                                                    </div>
                                                    <span class="mor-rte-sep"></span>
                                                    <div class="mor-rte-group" role="group" aria-label="Link">
                                                      <button type="button" data-cmd="createLink" title="Insert or edit link (Ctrl+K)" aria-label="Link">Link</button>
                                                      <button type="button" data-cmd="unlink" title="Remove link" aria-label="Remove link" class="mor-rte-quiet">Unlink</button>
                                                    </div>
                                                    <span class="mor-rte-sep"></span>
                                                    <div class="mor-rte-group" role="group" aria-label="History">
                                                      <button type="button" data-cmd="undo" title="Undo" aria-label="Undo">Undo</button>
                                                      <button type="button" data-cmd="redo" title="Redo" aria-label="Redo">Redo</button>
                                                      <button type="button" data-cmd="removeFormat" title="Clear formatting" aria-label="Clear formatting" class="mor-rte-quiet">Clear</button>
                                                    </div>
                                                    <span class="mor-rte-sep"></span>
                                                    <button type="button" data-rte-done class="mor-rte-primary" title="Save and close (Enter on single-line)">Done</button>
                                                    <button type="button" data-rte-cancel class="mor-rte-quiet" title="Discard changes (Esc)">Cancel</button>
                                                  </div>
                                                  <div class="mor-rte-link-row">
                                                    <label class="mor-rte-link-label">Link URL
                                                      <input type="text" data-link-url placeholder="/page.php or https://example.com" autocomplete="off" spellcheck="false" />
                                                    </label>
                                                    <label class="mor-rte-link-check" title="Open in a new browser tab">
                                                      <input type="checkbox" data-link-blank /> New tab
                                                    </label>
                                                    <button type="button" data-link-apply class="mor-rte-primary">Apply link</button>
                                                    <button type="button" data-link-cancel class="mor-rte-quiet">Close</button>
                                                  </div>
                                                  <div class="mor-rte-hint">Editing this block · select text to format · <strong>Done</strong> saves · <strong>Esc</strong> cancels</div>`;
                                                doc.body.appendChild(bar);
                                                placeRichToolbar(bar, el);
                                                // Keep text selection when clicking toolbar chrome
                                                bar.addEventListener('mousedown', e => {
                                                    if (e.target.closest('input,select,textarea,label')) return;
                                                    e.preventDefault();
                                                });
                                                const run = (cmd, val) => {
                                                    el.focus();
                                                    try {
                                                        if (cmd === 'formatBlock') {
                                                            const tag = (val || 'p').replace(/[<>]/g, '');
                                                            try { doc.execCommand('formatBlock', false, '<' + tag + '>'); }
                                                            catch (_) { doc.execCommand('formatBlock', false, tag); }
                                                        } else {
                                                            doc.execCommand(cmd, false, val || null);
                                                        }
                                                    } catch (_) {}
                                                    syncOnState();
                                                    placeRichToolbar(bar, el);
                                                };
                                                const linkAnchorFromSel = () => {
                                                    try {
                                                        const sel = doc.getSelection();
                                                        if (!sel || !sel.anchorNode) return null;
                                                        const n = sel.anchorNode;
                                                        return n.nodeType === 1
                                                            ? n.closest('a')
                                                            : (n.parentElement && n.parentElement.closest('a'));
                                                    } catch (_) { return null; }
                                                };
                                                const openLinkPanel = () => {
                                                    bar.classList.add('mor-rte-link-open');
                                                    const inp = bar.querySelector('[data-link-url]');
                                                    const blank = bar.querySelector('[data-link-blank]');
                                                    let href = '';
                                                    const a = linkAnchorFromSel();
                                                    if (a && a.getAttribute('href')) href = a.getAttribute('href');
                                                    if (blank) blank.checked = !!(a && a.getAttribute('target') === '_blank');
                                                    if (inp) {
                                                        inp.value = href || 'https://';
                                                        inp.focus();
                                                        inp.select();
                                                    }
                                                    placeRichToolbar(bar, el);
                                                };
                                                const syncOnState = () => {
                                                    bar.querySelectorAll('[data-cmd]').forEach(btn => {
                                                        const c = btn.getAttribute('data-cmd');
                                                        if (!c || c === 'createLink' || c === 'unlink' || c === 'removeFormat' || c === 'undo' || c === 'redo') return;
                                                        try {
                                                            btn.classList.toggle('is-on', !!doc.queryCommandState(c));
                                                            btn.setAttribute('aria-pressed', btn.classList.contains('is-on') ? 'true' : 'false');
                                                        } catch (_) {}
                                                    });
                                                    const blockSel = bar.querySelector('[data-block]');
                                                    if (blockSel) {
                                                        try {
                                                            let v = (doc.queryCommandValue('formatBlock') || '').toString().toLowerCase().replace(/[<>]/g, '');
                                                            if (!v || v === 'div') v = 'p';
                                                            if ([...blockSel.options].some(o => o.value === v)) blockSel.value = v;
                                                        } catch (_) {}
                                                    }
                                                };
                                                bar.querySelectorAll('[data-cmd]').forEach(btn => {
                                                    btn.addEventListener('click', e => {
                                                        e.preventDefault();
                                                        e.stopPropagation();
                                                        const cmd = btn.getAttribute('data-cmd');
                                                        if (cmd === 'createLink') { openLinkPanel(); return; }
                                                        run(cmd);
                                                    });
                                                });
                                                const blockSel = bar.querySelector('[data-block]');
                                                if (blockSel) blockSel.addEventListener('change', () => {
                                                    const v = blockSel.value;
                                                    if (!v) return;
                                                    run('formatBlock', v);
                                                });
                                                const applyLink = bar.querySelector('[data-link-apply]');
                                                const cancelLink = bar.querySelector('[data-link-cancel]');
                                                if (applyLink) applyLink.addEventListener('click', e => {
                                                    e.preventDefault();
                                                    const url = ((bar.querySelector('[data-link-url]') || {}).value || '').trim();
                                                    const blank = !!(bar.querySelector('[data-link-blank]') || {}).checked;
                                                    if (url) {
                                                        // If caret is collapsed inside a link, select the whole link first.
                                                        const a0 = linkAnchorFromSel();
                                                        if (a0) {
                                                            try {
                                                                const r = doc.createRange();
                                                                r.selectNodeContents(a0);
                                                                const s = doc.getSelection();
                                                                s.removeAllRanges();
                                                                s.addRange(r);
                                                            } catch (_) {}
                                                        }
                                                        run('createLink', url);
                                                        const a = linkAnchorFromSel();
                                                        if (a) {
                                                            a.setAttribute('href', url);
                                                            if (blank) { a.setAttribute('target', '_blank'); a.setAttribute('rel', 'noopener noreferrer'); }
                                                            else { a.removeAttribute('target'); a.removeAttribute('rel'); }
                                                        }
                                                    }
                                                    bar.classList.remove('mor-rte-link-open');
                                                    placeRichToolbar(bar, el);
                                                });
                                                if (cancelLink) cancelLink.addEventListener('click', e => {
                                                    e.preventDefault();
                                                    bar.classList.remove('mor-rte-link-open');
                                                    placeRichToolbar(bar, el);
                                                });
                                                const linkInp = bar.querySelector('[data-link-url]');
                                                if (linkInp) linkInp.addEventListener('keydown', e => {
                                                    if (e.key === 'Enter') { e.preventDefault(); applyLink && applyLink.click(); }
                                                    if (e.key === 'Escape') { e.preventDefault(); bar.classList.remove('mor-rte-link-open'); placeRichToolbar(bar, el); }
                                                });
                                                const doneBtn = bar.querySelector('[data-rte-done]');
                                                const cancelBtn = bar.querySelector('[data-rte-cancel]');
                                                if (doneBtn) doneBtn.addEventListener('click', e => {
                                                    e.preventDefault();
                                                    finishRichEdit(el, true);
                                                });
                                                if (cancelBtn) cancelBtn.addEventListener('click', e => {
                                                    e.preventDefault();
                                                    finishRichEdit(el, false);
                                                });
                                                doc.addEventListener('selectionchange', syncOnState);
                                                bar._cleanup = () => doc.removeEventListener('selectionchange', syncOnState);
                                                window.__morRichActive = el;
                                                syncOnState();
                                                // Reposition after layout settles
                                                requestAnimationFrame(() => placeRichToolbar(bar, el));
                                            }
                                            function insertAtCaret(html) {
                                                const el = window.__morRichActive;
                                                if (!el || el.contentEditable !== 'true') return false;
                                                el.focus();
                                                try {
                                                    if (doc.queryCommandSupported && doc.queryCommandSupported('insertHTML')) {
                                                        doc.execCommand('insertHTML', false, html);
                                                    } else {
                                                        const sel = doc.getSelection();
                                                        if (!sel || !sel.rangeCount) return false;
                                                        const range = sel.getRangeAt(0);
                                                        range.deleteContents();
                                                        const tmp = doc.createElement('div');
                                                        tmp.innerHTML = html;
                                                        const frag = doc.createDocumentFragment();
                                                        let node;
                                                        while ((node = tmp.firstChild)) frag.appendChild(node);
                                                        range.insertNode(frag);
                                                    }
                                                    return true;
                                                } catch (_) { return false; }
                                            }
                                            window.__morRichInsert = insertAtCaret;
                                            window.__morRichCmd = (cmd, val) => {
                                                const el = window.__morRichActive;
                                                if (!el || el.contentEditable !== 'true') return false;
                                                el.focus();
                                                try {
                                                    if (cmd === 'createLink') {
                                                        const bar = doc.getElementById('mor-rte-bar');
                                                        if (bar && !val) {
                                                            bar.classList.add('mor-rte-link-open');
                                                            const inp = bar.querySelector('[data-link-url]');
                                                            if (inp) { inp.focus(); inp.select(); }
                                                            return true;
                                                        }
                                                        const url = val || prompt('Link URL (https://… or /page.php)', 'https://');
                                                        if (url) doc.execCommand('createLink', false, url);
                                                    } else if (cmd === 'formatBlock') {
                                                        const tag = (val || 'p').replace(/[<>]/g, '');
                                                        try { doc.execCommand('formatBlock', false, '<' + tag + '>'); }
                                                        catch (_) { doc.execCommand('formatBlock', false, tag); }
                                                    } else {
                                                        doc.execCommand(cmd, false, val || null);
                                                    }
                                                    return true;
                                                } catch (_) { return false; }
                                            };
                                            // Drop Insert-dock blocks / images onto the page (Google Sites–like).
                                            function nearestBlock(el) {
                                                if (!el || el === doc.body) return null;
                                                return el.closest('[data-mor-block],p,h1,h2,h3,h4,h5,h6,li,blockquote,div,section,article,main,.canvas-core,hr,ul,ol');
                                            }
                                            function contentHostOf(el) {
                                                if (!el) return contentHosts(doc)[0] || doc.body;
                                                return el.closest('.canvas-core, main, article, .content, #content') || el.parentElement || doc.body;
                                            }
                                            /** Insert HTML; return {ok, oldHtml, newHtml} for PAGE_TEXT_EDIT when possible. */
                                            function insertHtmlNear(target, html, before) {
                                                const wrap = doc.createElement('div');
                                                wrap.innerHTML = html;
                                                const nodes = Array.from(wrap.childNodes);
                                                if (!nodes.length) return { ok: false };
                                                if (target && target !== doc.body && target.parentNode) {
                                                    const oldHtml = target.outerHTML;
                                                    nodes.forEach(n => {
                                                        if (before) target.parentNode.insertBefore(n, target);
                                                        else if (target.nextSibling) target.parentNode.insertBefore(n, target.nextSibling);
                                                        else target.parentNode.appendChild(n);
                                                    });
                                                    // Reconstruct: target still in place; siblings were inserted.
                                                    // Save as old block → block + inserted HTML (or reverse if before).
                                                    const newHtml = before ? (html + oldHtml) : (oldHtml + html);
                                                    markContentBlocks(doc);
                                                    return { ok: true, oldHtml, newHtml };
                                                }
                                                const host = contentHostOf(target);
                                                if (!host) return { ok: false };
                                                const beforeInner = host.innerHTML;
                                                nodes.forEach(n => host.appendChild(n));
                                                const afterInner = host.innerHTML;
                                                markContentBlocks(doc);
                                                if (beforeInner === afterInner) return { ok: true };
                                                return { ok: true, oldHtml: beforeInner, newHtml: afterInner };
                                            }
                                            function persistDomEdit(oldHtml, newHtml) {
                                                if (!oldHtml || !newHtml || oldHtml === newHtml) return;
                                                // Cap huge host dumps — page_edit has its own size guard too.
                                                if (oldHtml.length > 24000 || newHtml.length > 32000) {
                                                    dioxus.send({ action: 'RICH_DROP_HINT', message: 'Change too large to auto-save — use Code view.' });
                                                    return;
                                                }
                                                dioxus.send({ action: 'PAGE_TEXT_EDIT', old_text: oldHtml, new_text: newHtml, rich: true });
                                            }
                                            window.__morRichDropHtml = function(html, clientX, clientY) {
                                                if (!html) return false;
                                                if (window.__morRichActive && window.__morRichActive.contentEditable === 'true') {
                                                    return insertAtCaret(html);
                                                }
                                                let target = null;
                                                try {
                                                    if (typeof clientX === 'number' && doc.elementFromPoint) {
                                                        target = nearestBlock(doc.elementFromPoint(clientX, clientY));
                                                    }
                                                } catch (_) {}
                                                const res = insertHtmlNear(target, html, false);
                                                if (res.ok && res.oldHtml && res.newHtml) persistDomEdit(res.oldHtml, res.newHtml);
                                                else if (res.ok) dioxus.send({ action: 'RICH_DROP_HINT', message: 'Block inserted in preview — open Code view if it did not save.' });
                                                return !!res.ok;
                                            };
                                            doc.addEventListener('dragover', e => {
                                                if (window.__morEditActive === false) return;
                                                const types = e.dataTransfer && e.dataTransfer.types ? Array.from(e.dataTransfer.types) : [];
                                                if (contentDragEl || types.includes('Files') || types.includes('text/uri-list') || types.includes('text/plain') || types.includes('application/x-mor-insert-html') || types.includes('application/x-mor-content-block')) {
                                                    e.preventDefault();
                                                    e.dataTransfer.dropEffect = contentDragEl ? 'move' : 'copy';
                                                    const t = nearestBlock(e.target);
                                                    doc.querySelectorAll('.mor-drop-hover').forEach(x => x.classList.remove('mor-drop-hover'));
                                                    if (t && t !== contentDragEl) t.classList.add('mor-drop-hover');
                                                }
                                            });
                                            doc.addEventListener('dragleave', e => {
                                                const t = nearestBlock(e.target);
                                                if (t) t.classList.remove('mor-drop-hover');
                                            });
                                            doc.addEventListener('drop', e => {
                                                if (window.__morEditActive === false) return;
                                                doc.querySelectorAll('.mor-drop-hover').forEach(x => x.classList.remove('mor-drop-hover'));
                                                // Reorder content blocks (drag handle on [data-mor-block]).
                                                if (contentDragEl) {
                                                    e.preventDefault();
                                                    e.stopPropagation();
                                                    const dest = nearestBlock(e.target);
                                                    const from = contentDragEl;
                                                    contentDragEl = null;
                                                    from.classList.remove('dragging');
                                                    if (dest && dest !== from && dest.parentNode && from.parentNode === dest.parentNode) {
                                                        const parent = from.parentNode;
                                                        const beforeHtml = parent.innerHTML;
                                                        const rect = dest.getBoundingClientRect();
                                                        const before = e.clientY < rect.top + rect.height / 2;
                                                        parent.insertBefore(from, before ? dest : dest.nextSibling);
                                                        const afterHtml = parent.innerHTML;
                                                        persistDomEdit(beforeHtml, afterHtml);
                                                    }
                                                    return;
                                                }
                                                let html = '';
                                                try {
                                                    html = e.dataTransfer.getData('application/x-mor-insert-html') || '';
                                                    if (!html) {
                                                        const plain = e.dataTransfer.getData('text/plain') || '';
                                                        if (plain.startsWith('MOR_INSERT_HTML:')) html = plain.slice('MOR_INSERT_HTML:'.length);
                                                    }
                                                } catch (_) {}
                                                // External image URL drag
                                                if (!html) {
                                                    const uri = (e.dataTransfer.getData('text/uri-list') || '').split('\n').map(s => s.trim()).find(s => s && !s.startsWith('#'));
                                                    if (uri && /^https?:\/\//i.test(uri)) {
                                                        html = '<img src="' + uri.replace(/"/g, '&quot;') + '" alt="" style="max-width:100%;height:auto" />';
                                                    }
                                                }
                                                if (html) {
                                                    e.preventDefault();
                                                    e.stopPropagation();
                                                    const t = nearestBlock(e.target);
                                                    if (window.__morRichActive && window.__morRichActive.contentEditable === 'true') {
                                                        insertAtCaret(html);
                                                    } else {
                                                        const res = insertHtmlNear(t, html, false);
                                                        if (res.ok && res.oldHtml && res.newHtml) persistDomEdit(res.oldHtml, res.newHtml);
                                                        else dioxus.send({ action: 'RICH_DROP_HINT', message: 'Block dropped — could not auto-save; use Code view if needed.' });
                                                    }
                                                    return;
                                                }
                                                // File image drop handled existing path for SVG icons; also handle raster images.
                                                if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
                                                    const file = e.dataTransfer.files[0];
                                                    if (file && file.type && file.type.startsWith('image/') && !file.type.includes('svg')) {
                                                        e.preventDefault();
                                                        // Host app imports via path only on desktop file dialog; browser File has no path.
                                                        // Read as data URL for immediate preview; user can re-import via Insert dock to file.
                                                        const reader = new FileReader();
                                                        reader.onload = (ev) => {
                                                            const dataUrl = ev.target.result;
                                                            const alt = (file.name || 'image').replace(/"/g, '');
                                                            const imgHtml = '<img src="' + dataUrl + '" alt="' + alt + '" style="max-width:100%;height:auto" />';
                                                            if (window.__morRichActive && window.__morRichActive.contentEditable === 'true') insertAtCaret(imgHtml);
                                                            else {
                                                                const res = insertHtmlNear(nearestBlock(e.target), imgHtml, false);
                                                                if (res.ok && res.oldHtml && res.newHtml) persistDomEdit(res.oldHtml, res.newHtml);
                                                            }
                                                        };
                                                        reader.readAsDataURL(file);
                                                    }
                                                }
                                            });
                                            // Content-block drag start (reorder). Skip while text-editing.
                                            doc.addEventListener('dragstart', e => {
                                                if (window.__morEditActive === false) return;
                                                if (e.target.closest && e.target.closest('#mor-rte-bar')) { e.preventDefault(); return; }
                                                const el = e.target.closest && e.target.closest('[data-mor-block]');
                                                if (!el || el.getAttribute('data-block-id')) return;
                                                if (el.contentEditable === 'true' || el.closest('[contenteditable="true"]')) return;
                                                contentDragEl = el;
                                                el.classList.add('dragging');
                                                try {
                                                    e.dataTransfer.effectAllowed = 'move';
                                                    e.dataTransfer.setData('application/x-mor-content-block', '1');
                                                    e.dataTransfer.setData('text/plain', 'MOR_CONTENT_BLOCK');
                                                } catch (_) {}
                                            });
                                            doc.addEventListener('dragend', e => {
                                                if (contentDragEl) contentDragEl.classList.remove('dragging');
                                                contentDragEl = null;
                                                doc.querySelectorAll('.mor-drop-hover').forEach(x => x.classList.remove('mor-drop-hover'));
                                            });
                                            function selLabel(el) {
                                                if (!el || el === doc.body || el === doc.documentElement) return 'Body';
                                                const tag = (el.tagName || 'el').toLowerCase();
                                                if (el.getAttribute('data-field-path') || el.getAttribute('data-mor-edit')) {
                                                    return 'Field · ' + (el.getAttribute('data-field-path') || el.getAttribute('data-mor-edit'));
                                                }
                                                if (el.getAttribute('data-block-id')) return 'Widget · ' + el.getAttribute('data-block-id');
                                                if (el.getAttribute('data-edit-target')) return 'Token · ' + el.getAttribute('data-edit-target');
                                                if (el.id) return tag.toUpperCase() + ' · #' + el.id;
                                                const cls = (el.className && el.className.baseVal !== undefined ? el.className.baseVal : el.className) || '';
                                                const first = String(cls).trim().split(/\s+/).filter(Boolean)[0];
                                                if (first) return tag.toUpperCase() + ' · .' + first;
                                                const pretty = {h1:'Heading 1',h2:'Heading 2',h3:'Heading 3',h4:'Heading 4',h5:'Heading 5',h6:'Heading 6',p:'Paragraph',a:'Link',nav:'Nav',button:'Button',img:'Image',li:'List item',ul:'List',ol:'List',header:'Header',footer:'Footer',main:'Main',section:'Section',article:'Article',span:'Text',div:'Div',aside:'Aside',figure:'Figure',figcaption:'Caption'};
                                                return pretty[tag] || tag.toUpperCase();
                                            }
                                            function clearSelection(doc) {
                                                doc.querySelectorAll('.mor-canvas-selected').forEach(el => {
                                                    el.classList.remove('mor-canvas-selected');
                                                    el.removeAttribute('data-mor-sel-label');
                                                });
                                            }
                                            function selectEl(doc, el) {
                                                if (!el || el === doc.body || el === doc.documentElement) return;
                                                clearSelection(doc);
                                                el.classList.add('mor-canvas-selected');
                                                el.setAttribute('data-mor-sel-label', selLabel(el));
                                                doc.__morSel = el;
                                            }

                                            // Inspect hover: one soft outline under the cursor
                                            doc.addEventListener('mouseover', e => {
                                                if (!window.__morXrayActive) return;
                                                if (doc.__morHov) { doc.__morHov.classList.remove('mor-xray-hover'); doc.__morHov = null; }
                                                let t = e.target;
                                                if (!t || t === doc.body || t === doc.documentElement) return;
                                                if (t.nodeType !== 1) t = t.parentElement;
                                                if (!t || t.classList.contains('mor-canvas-selected')) return;
                                                doc.__morHov = t;
                                                t.classList.add('mor-xray-hover');
                                            });
                                            // Double-click text → edit with floating toolbar.
                                            // Select the word under the caret (not the whole block).
                                            doc.addEventListener('dblclick', e => {
                                                if (window.__morEditActive === false) return;
                                                if (e.target.closest('#mor-rte-bar')) return;
                                                // Prefer a real text block over a huge wrapping div.
                                                let el = e.target.closest('[data-field-path],[data-mor-edit]');
                                                if (!el) {
                                                    el = e.target.closest('h1,h2,h3,h4,h5,h6,p,li,blockquote,figcaption,td,th,label,a');
                                                }
                                                if (!el) el = e.target.closest(TEXT_EDIT);
                                                if (!el || el.closest('script,style,svg,code,pre,input,textarea,select,#mor-rte-bar,nav,header,footer,.mor-panel')) return;
                                                // Avoid turning the whole main column into contentEditable.
                                                if (el.matches('div') && (el.classList.contains('canvas-core') || el.id === 'page-wrap' || el.children.length > 3)) {
                                                    el = e.target.closest('p,h1,h2,h3,h4,h5,h6,li,blockquote') || el;
                                                }
                                                e.preventDefault();
                                                e.stopPropagation();
                                                // End any previous edit session first.
                                                if (window.__morRichActive && window.__morRichActive !== el) {
                                                    finishRichEdit(window.__morRichActive, true);
                                                }
                                                selectEl(doc, el);
                                                el.setAttribute('data-mor-edit-before', el.innerText);
                                                el.setAttribute('data-mor-edit-before-html', el.innerHTML);
                                                el.contentEditable = 'true';
                                                el.classList.add('mor-canvas-editing');
                                                el.setAttribute('spellcheck', 'true');
                                                el.focus();
                                                try {
                                                    // Prefer the browser's word selection from the dblclick;
                                                    // only fall back to placing the caret if nothing is selected.
                                                    const s = doc.getSelection();
                                                    if (!s || s.isCollapsed || !el.contains(s.anchorNode)) {
                                                        const r = doc.createRange();
                                                        r.selectNodeContents(el);
                                                        r.collapse(false); // caret at end
                                                        s.removeAllRanges();
                                                        s.addRange(r);
                                                    }
                                                } catch (_) {}
                                                showRichToolbar(el);
                                            });
                                            doc.addEventListener('contextmenu', e => {
                                                const targetEl = e.target.closest("[data-edit-target^='icons.']");
                                                const textTarget = e.target.closest('h1, h2, h3, h4, h5, h6, p, span, a');
                                                if (targetEl || textTarget) {
                                                    e.preventDefault();
                                                    e.stopPropagation();
                                                    const frm = document.getElementById(FID);
                                                    const rect = frm ? frm.getBoundingClientRect() : { left: 0, top: 0 };
                                                    const wrapper = document.querySelector('.preview-scale-wrapper');
                                                    const scale = wrapper ? parseFloat(getComputedStyle(wrapper).getPropertyValue('--preview-scale')) || 1 : 1;
                                                    const x = rect.left + e.clientX * scale;
                                                    const y = rect.top + e.clientY * scale;
                                                    if (targetEl) {
                                                        dioxus.send({
                                                            action: "svg_context_menu",
                                                            kind: "svg",
                                                            target_id: targetEl.getAttribute("data-edit-target"),
                                                            x: x,
                                                            y: y
                                                        });
                                                    } else if (textTarget) {
                                                        dioxus.send({
                                                            action: "svg_context_menu",
                                                            kind: "preview_typography",
                                                            target_id: textTarget.tagName.toLowerCase(),
                                                            x: x,
                                                            y: y
                                                        });
                                                    }
                                                }
                                            });
                                            doc.addEventListener('blur', e => {
                                                // Don't end edit when focusing the floating toolbar / link field.
                                                const related = e.relatedTarget;
                                                if (related && related.closest && related.closest('#mor-rte-bar')) return;
                                                const el = e.target.closest('[contenteditable="true"],[contenteditable=true]');
                                                if (!el || el.contentEditable !== "true") return;
                                                if (el.id === 'mor-rte-bar' || el.closest('#mor-rte-bar')) return;
                                                setTimeout(() => {
                                                    if (doc.activeElement && doc.activeElement.closest && doc.activeElement.closest('#mor-rte-bar')) return;
                                                    if (el.contentEditable !== 'true') return;
                                                    finishRichEdit(el, true);
                                                }, 160);
                                            }, true);
                                            // Formatting shortcuts + Done/Cancel
                                            doc.addEventListener('keydown', e => {
                                                // Link field shortcuts handled on the input itself.
                                                if (e.target && e.target.closest && e.target.closest('#mor-rte-bar')) return;
                                                const el = doc.activeElement;
                                                if (!el || el.contentEditable !== 'true') return;
                                                if ((e.ctrlKey || e.metaKey) && !e.altKey) {
                                                    const k = (e.key || '').toLowerCase();
                                                    if (k === 'b') { e.preventDefault(); doc.execCommand('bold'); return; }
                                                    if (k === 'i') { e.preventDefault(); doc.execCommand('italic'); return; }
                                                    if (k === 'u') { e.preventDefault(); doc.execCommand('underline'); return; }
                                                    if (k === 'k') {
                                                        e.preventDefault();
                                                        if (window.__morRichCmd) window.__morRichCmd('createLink');
                                                        return;
                                                    }
                                                    if (k === 's') {
                                                        e.preventDefault();
                                                        finishRichEdit(el, true);
                                                        return;
                                                    }
                                                }
                                                if (e.key === 'Escape') {
                                                    e.preventDefault();
                                                    finishRichEdit(el, false);
                                                } else if (e.key === 'Enter' && !e.shiftKey && !/^(H[1-6]|P|LI|DIV|BLOCKQUOTE)$/i.test(el.tagName)) {
                                                    e.preventDefault();
                                                    finishRichEdit(el, true);
                                                }
                                            });
                                            let draggedId = null;
                                            doc.addEventListener('dragstart', e => {
                                                if (window.__morEditActive === false) { e.preventDefault(); return; }
                                                const el = e.target.closest('[data-block-id]');
                                                if (el) { draggedId = el.getAttribute('data-block-id'); e.dataTransfer.effectAllowed = 'move'; el.classList.add('dragging'); }
                                            });
                                            doc.addEventListener('dragend', e => e.target.closest('[data-block-id]')?.classList.remove('dragging'));
                                            doc.addEventListener('dragover', e => {
                                                if (e.dataTransfer.types.includes('Files')) { e.preventDefault(); e.dataTransfer.dropEffect = 'copy'; return; }
                                                e.preventDefault();
                                                const el = e.target.closest('[data-block-id]');
                                                if (el && el.getAttribute('data-block-id') !== draggedId) { el.classList.add('drag-over'); e.dataTransfer.dropEffect = 'move'; }
                                            });
                                            doc.addEventListener('dragleave', e => e.target.closest('[data-block-id]')?.classList.remove('drag-over'));
                                            doc.addEventListener('drop', e => {
                                                if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
                                                    e.preventDefault();
                                                    if (window.__morEditActive === false) return;
                                                    const file = e.dataTransfer.files[0];
                                                    if (file.type.includes("svg") || file.name.endsWith(".svg")) {
                                                        const targetEl = e.target.closest('[data-edit-target^="icons."]');
                                                        if (targetEl) {
                                                            const targetName = targetEl.getAttribute('data-edit-target');
                                                            const reader = new FileReader();
                                                            reader.onload = (event) => {
                                                                dioxus.send({ action: "DROP_SVG", target: targetName, content: event.target.result });
                                                            };
                                                            reader.readAsText(file);
                                                        }
                                                    }
                                                    return;
                                                }
                                                e.preventDefault();
                                                const el = e.target.closest('[data-block-id]');
                                                if (el) {
                                                    el.classList.remove('drag-over');
                                                    const destId = el.getAttribute('data-block-id');
                                                    if (draggedId && draggedId !== destId)
                                                        dioxus.send({action: "MOVE_WIDGET", id: draggedId, dest: destId});
                                                }
                                                draggedId = null;
                                            });
                                            doc.addEventListener('click', e => {
                                                // 1. Icon Edit (Shift+Click)
                                                if (e.shiftKey && window.__morEditActive !== false) {
                                                    const targetEl = e.target.closest("[data-edit-target^='icons.']"); 
                                                    if (targetEl) { 
                                                        e.preventDefault(); 
                                                        dioxus.send({action: "ICON_EDIT", target: targetEl.getAttribute("data-edit-target")}); 
                                                        return; 
                                                    } 
                                                }
                                                
                                                // 2. X-Ray Selection Bridge — Inspect/Edit: Webflow-like
                                                // click-to-select any node; Browse lets links navigate.
                                                if (window.__morXrayActive) {
                                                    // Don't steal clicks while typing in a contentEditable
                                                    if (e.target.isContentEditable || e.target.closest('[contenteditable="true"]')) return;
                                                    e.preventDefault();
                                                    e.stopPropagation();
                                                    let pick = e.target.closest('[data-edit-target], [data-field-path], [data-mor-edit], [data-block-id]');
                                                    if (!pick) {
                                                        pick = e.target.closest(TEXT_EDIT + ',div,section,article,nav,header,footer,aside,main,img,figure');
                                                    }
                                                    if (!pick || pick === doc.body || pick === doc.documentElement) {
                                                        pick = e.target.nodeType === 1 ? e.target : e.target.parentElement;
                                                    }
                                                    if (pick && pick !== doc.body && pick !== doc.documentElement) {
                                                        // Sidebar nav: always select the <a.mor-sidebar__item>
                                                        // so href/group/item indices are available for editing.
                                                        const navItem = pick.closest
                                                            ? pick.closest('a.mor-sidebar__item, .mor-sidebar__item')
                                                            : null;
                                                        if (navItem) pick = navItem;
                                                        selectEl(doc, pick);
                                                        const targetId = pick.getAttribute('data-edit-target')
                                                            || pick.getAttribute('data-field-path')
                                                            || pick.getAttribute('data-mor-edit')
                                                            || pick.getAttribute('data-block-id');
                                                        if (targetId) {
                                                            dioxus.send({action: "SELECT", target: targetId});
                                                        } else {
                                                            const cls = pick.className;
                                                            const labelEl = pick.querySelector
                                                                ? pick.querySelector('.mor-sidebar__item-label')
                                                                : null;
                                                            // Prefer <a>/<img>/<button> for instance fields when nested.
                                                            let inst = pick;
                                                            if (pick.closest) {
                                                                const a = pick.closest('a[href]');
                                                                const img = pick.closest('img');
                                                                const btn = pick.closest('button, a.btn, a.btn-primary, .btn-primary');
                                                                if (img && pick.tagName && pick.tagName.toLowerCase() !== 'a') inst = img;
                                                                else if (a) inst = a;
                                                                else if (btn) inst = btn;
                                                            }
                                                            const outer = (inst.outerHTML || '').slice(0, 8000);
                                                            dioxus.send({
                                                                action: "SELECT_DOM",
                                                                tag: inst.tagName.toLowerCase(),
                                                                classes: (function(){
                                                                    const c = inst.className;
                                                                    return (c && c.baseVal !== undefined ? c.baseVal : c) || '';
                                                                })(),
                                                                label: selLabel(inst),
                                                                href: inst.getAttribute && inst.getAttribute('href') || '',
                                                                src: inst.getAttribute && inst.getAttribute('src') || '',
                                                                alt: inst.getAttribute && inst.getAttribute('alt') || '',
                                                                text: (labelEl && labelEl.innerText)
                                                                    || (inst.innerText || '').trim().slice(0, 80),
                                                                outer_html: outer,
                                                                nav_group: pick.getAttribute && pick.getAttribute('data-group-index'),
                                                                nav_item: pick.getAttribute && pick.getAttribute('data-item-index')
                                                            });
                                                        }
                                                    }
                                                    return;
                                                }

                                                // 3. Theme Toggle (Browse only from here down)
                                                const t = e.target, btn = t.closest('#mor-theme-toggle'), a = t.closest('a');
                                                if (btn) { e.preventDefault(); dioxus.send({action: "TOGGLE_DARK_MODE"}); return; }

                                                // 4. Navigation
                                                if (a && (a.getAttribute('href')||'').match(/^[\/#]/)) {
                                                    e.preventDefault();
                                                    const href = a.getAttribute('href');
                                                    // In-page fragments (back-to-top, TOC jumps) scroll inside the
                                                    // iframe like they will on the real blog, instead of re-rendering.
                                                    if (href.length > 1 && href[0] === '#') {
                                                        const frag = doc.getElementById(href.slice(1));
                                                        if (frag) { frag.scrollIntoView(); return; }
                                                    }
                                                    dioxus.send({action: "NAVIGATE", target: href});
                                                    return;
                                                }
                                            });
                                            applyXray(doc, window.__morXrayActive);
                                        }
                                        function install() {
                                            const src = document.getElementById(SID), frm = document.getElementById(FID);
                                            if (!src || !frm) return setTimeout(install, 50);
                                            new MutationObserver(() => write(src, frm)).observe(src, {childList:true, characterData:true, subtree:true});
                                            write(src, frm);
                                        }
                                        install();
                                    })();
                                    "#
                                );

                                while let Ok(json) = eval.recv::<serde_json::Value>().await {
                                    if let Some(action) = json.get("action").and_then(|a| a.as_str()) {
                                        match action {
                                            "SELECT" => {
                                                if let Some(target) = json.get("target").and_then(|t| t.as_str()) {
                                                    if let Some(handler) = on_select.as_ref() { handler.call(target.to_string()); }
                                                }
                                            }
                                            "SELECT_DOM" => {
                                                if let Some(tag) = json.get("tag").and_then(|t| t.as_str()) {
                                                    let classes = json.get("classes").and_then(|c| c.as_str()).unwrap_or("");
                                                    let opt_str = |k: &str| {
                                                        json.get(k)
                                                            .and_then(|h| h.as_str())
                                                            .filter(|s| !s.is_empty())
                                                            .map(str::to_string)
                                                    };
                                                    let parse_idx = |k: &str| {
                                                        json.get(k).and_then(|v| {
                                                            v.as_u64()
                                                                .map(|n| n as usize)
                                                                .or_else(|| {
                                                                    v.as_str().and_then(|s| s.parse().ok())
                                                                })
                                                        })
                                                    };
                                                    let facts = crate::app::edit_context::DomSelectFacts {
                                                        tag: tag.to_string(),
                                                        classes: classes.to_string(),
                                                        href: opt_str("href"),
                                                        src: opt_str("src"),
                                                        alt: opt_str("alt"),
                                                        text: opt_str("text"),
                                                        outer_html: opt_str("outer_html"),
                                                        nav_group: parse_idx("nav_group"),
                                                        nav_item: parse_idx("nav_item"),
                                                    };
                                                    if let Some(handler) = on_select_dom.as_ref() {
                                                        handler.call(facts);
                                                    }
                                                }
                                            }
                                            "NAVIGATE" => {
                                                if let Some(target) = json.get("target").and_then(|t| t.as_str()) {
                                                    if let Some(handler) = on_navigate.as_ref() { handler.call(target.to_string()); }
                                                }
                                            }
                                            "ICON_EDIT" => {
                                                if let Some(target) = json.get("target").and_then(|t| t.as_str()) {
                                                    if let Some(handler) = on_icon_edit.as_ref() { handler.call(target.to_string()); }
                                                }
                                            }
                                            "UPDATE_VALUE" => {
                                                if let (Some(target), Some(val)) = (json.get("target").and_then(|t| t.as_str()), json.get("value").and_then(|v| v.as_str())) {
                                                    if let Some(handler) = on_update_value.as_ref() { handler.call((target.to_string(), val.to_string())); }
                                                }
                                            }
                                            "PAGE_TEXT_EDIT" => {
                                                if let (Some(old_t), Some(new_t)) = (
                                                    json.get("old_text").and_then(|t| t.as_str()),
                                                    json.get("new_text").and_then(|t| t.as_str()),
                                                ) {
                                                    if let Some(handler) = on_page_text_edit.as_ref() {
                                                        handler.call((old_t.to_string(), new_t.to_string()));
                                                    }
                                                }
                                            }
                                            "RICH_DROP_HINT" => {
                                                if let Some(msg) = json.get("message").and_then(|m| m.as_str()) {
                                                    log::info!("Rich drop: {msg}");
                                                    if let Some(handler) = on_status.as_ref() {
                                                        handler.call(msg.to_string());
                                                    }
                                                }
                                            }
                                            "MOVE_WIDGET" => {
                                                if let (Some(id), Some(dest)) = (json.get("id").and_then(|i| i.as_str()), json.get("dest").and_then(|d| d.as_str())) {
                                                    if let Some(handler) = on_move_widget.as_ref() { handler.call((id.to_string(), dest.to_string())); }
                                                }
                                            }
                                            "TOGGLE_DARK_MODE" => {
                                                if let Some(handler) = on_toggle_dark_mode.as_ref() { handler.call(()); }
                                            }
                                            "DROP_SVG" => {
                                                if let (Some(target), Some(content)) = (json.get("target").and_then(|t| t.as_str()), json.get("content").and_then(|c| c.as_str())) {
                                                    if let Some(handler) = on_drop_svg.as_ref() { handler.call((target.to_string(), content.to_string())); }
                                                }
                                            }
                                            "svg_context_menu" | "ICON_CONTEXT_MENU" => {
                                                if let (Some(target), Some(x_val), Some(y_val)) = (
                                                    json.get("target_id").or_else(|| json.get("target")).and_then(|t| t.as_str()),
                                                    json.get("x").and_then(|x| x.as_f64()),
                                                    json.get("y").and_then(|y| y.as_f64()),
                                                ) {
                                                    let kind_str = json.get("kind").and_then(|k| k.as_str()).unwrap_or("svg").to_string();
                                                    if let Some(handler) = on_icon_context_menu.as_ref() {
                                                        handler.call(ContextMenuPayload {
                                                            x: x_val,
                                                            y: y_val,
                                                            kind: kind_str,
                                                            target_id: target.to_string(),
                                                        });
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            });
                        }
                    }

                    // Drag-to-resize handle on the frame's right edge (fixed-width modes only).
                    if current_viewport != PreviewViewport::Fit {
                        div {
                            class: "preview-resize-handle",
                            title: "Drag to resize preview width",
                            onmousedown: move |e: MouseEvent| {
                                if e.trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
                                    drag_start.set((e.client_coordinates().x, preview_width()));
                                    dragging.set(true);
                                }
                            },
                            div { class: "preview-resize-handle-grip" }
                        }
                    }
                }
            }

            // While dragging, an overlay above the iframe keeps mousemove/up flowing
            // (iframes swallow mouse events) so the drag never gets stuck.
            if dragging() {
                div {
                    class: "preview-resize-overlay",
                    onmousemove: move |e: MouseEvent| {
                        let (start_x, start_w) = drag_start();
                        let dx = e.client_coordinates().x - start_x;
                        // ponytail: 2x because the frame is center-anchored, so the right
                        // edge moves dx for every 2*dx of width. Ignores --preview-scale,
                        // which is only <1 when the frame is already wider than the viewport.
                        let new_w = clamp_preview_width((start_w as f64 + dx * 2.0).round() as u32);
                        preview_width.set(new_w);
                        preview_viewport.set(PreviewViewport::Custom);
                    },
                    onmouseup: move |_| dragging.set(false),
                    onmouseleave: move |_| dragging.set(false),
                }
            }

            script { dangerous_inner_html: "{SCALER_JS}" }
        }
    }
}
