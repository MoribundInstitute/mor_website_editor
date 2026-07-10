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
    /// Edit mode (Browse | Inspect | Edit): outlines + selection ride
    /// `xray_active`; destructive-ish gestures (text dblclick, widget drag,
    /// SVG drop, icon shift-click) additionally require this.
    #[props(default)] edit_active: Option<Signal<bool>>,
    #[props(default)] on_navigate: Option<EventHandler<String>>,
    #[props(default)] on_select: Option<EventHandler<String>>,
    /// X-Ray click on a node with no marker anywhere up its chain: DOM facts
    /// (tag/classes/href/nav indices) — classification in Rust (edit_context).
    #[props(default)] on_select_dom: Option<EventHandler<crate::app::edit_context::DomSelectFacts>>,
    /// Last analyzed selection, rendered as the inspect chip overlay.
    #[props(default)] active_selection: Option<Signal<Option<SelectionInfo>>>,
    #[props(default)] on_icon_edit: Option<EventHandler<String>>,
    #[props(default)] on_icon_context_menu: Option<EventHandler<ContextMenuPayload>>,
    #[props(default)] on_update_value: Option<EventHandler<(String, String)>>,
    /// Unbound page-content edit: (old_text, new_text) → unique replace in page file.
    #[props(default)] on_page_text_edit: Option<EventHandler<(String, String)>>,
    #[props(default)] on_move_widget: Option<EventHandler<(String, String)>>,
    #[props(default)] on_toggle_dark_mode: Option<EventHandler<()>>,
    #[props(default)] on_drop_svg: Option<EventHandler<(String, String)>>,
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
    let current_viewport = preview_viewport();
    let viewport_label = current_viewport.label();
    let viewport_meta = if current_viewport == PreviewViewport::Fit {
        "Fit · available width".to_string()
    } else {
        format!("{} · {}px wide", viewport_label, preview_width())
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

            // Inspect chip: what the last X-Ray click resolved to — bound
            // config path, token surface, or "no binding" (inspect only).
            if xray_on {
                if let Some(sel) = active_selection.and_then(|s| s()) {
                    div {
                        class: if sel.context == EditContext::CodeOnly { "preview-selection-chip preview-selection-chip-unbound" } else { "preview-selection-chip" },
                        "{sel.label}"
                    }
                }
            }

            if xray_on {
                div {
                    class: "preview-xray-legend",
                    span { class: "preview-xray-legend-title", "X-Ray" }
                    span { class: "preview-xray-chip preview-xray-chip-layout", "Layout" }
                    span { class: "preview-xray-chip preview-xray-chip-widget", "Widget · drag" }
                    span { class: "preview-xray-chip preview-xray-chip-text", "Text · dbl-click" }
                    span { class: "preview-xray-chip preview-xray-chip-token", "Theme token" }
                    span { class: "preview-xray-chip preview-xray-chip-icon", "Icon · shift-click" }
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
                                        const XRAY_CSS = `
html.mor-xray-on .main-header{outline:2px solid #f59e0b;outline-offset:-2px;background:rgba(245,158,11,.07)!important}
html.mor-xray-on .mor-workspace{outline:2px solid #6366f1;outline-offset:-2px;background:rgba(99,102,241,.05)!important}
html.mor-xray-on .mor-panel.panel-left{outline:2px solid #10b981;outline-offset:-2px;background:rgba(16,185,129,.07)!important}
html.mor-xray-on .mor-panel.panel-right{outline:2px solid #14b8a6;outline-offset:-2px;background:rgba(20,184,166,.07)!important}
html.mor-xray-on .canvas-core{outline:2px solid #8b5cf6;outline-offset:-2px;background:rgba(139,92,246,.06)!important}
html.mor-xray-on .mor-footer{outline:2px solid #eab308;outline-offset:-2px;background:rgba(234,179,8,.06)!important}
html.mor-xray-on .main-header::before,html.mor-xray-on .mor-workspace::before,html.mor-xray-on .mor-panel.panel-left::before,html.mor-xray-on .mor-panel.panel-right::before,html.mor-xray-on .canvas-core::before,html.mor-xray-on .mor-footer::before{position:absolute;top:4px;left:4px;z-index:10000;padding:2px 6px;border-radius:3px;font:600 10px/1.3 ui-monospace,monospace;letter-spacing:.03em;text-transform:uppercase;pointer-events:none}
html.mor-xray-on .main-header::before{content:"Header";background:#f59e0b;color:#451a03}
html.mor-xray-on .mor-workspace::before{content:"Workspace";background:#6366f1;color:#1e1b4b}
html.mor-xray-on .mor-panel.panel-left::before{content:"Left sidebar";background:#10b981;color:#052e16}
html.mor-xray-on .mor-panel.panel-right::before{content:"Right sidebar";background:#14b8a6;color:#042f2e}
html.mor-xray-on .canvas-core::before{content:"Main content";background:#8b5cf6;color:#2e1065}
html.mor-xray-on .mor-footer::before{content:"Footer";background:#eab308;color:#422006}
html.mor-xray-on .main-header,html.mor-xray-on .mor-workspace,html.mor-xray-on .mor-panel,html.mor-xray-on .canvas-core,html.mor-xray-on .mor-footer{position:relative}
html.mor-xray-on [data-block-id]{outline:2px solid #22c55e!important;outline-offset:2px;background:rgba(34,197,94,.09)!important;position:relative;cursor:grab}
html.mor-xray-on [data-block-id]::before{content:attr(data-xray-widget);position:absolute;top:0;right:0;z-index:10001;padding:2px 6px;font:600 10px/1.3 ui-monospace,monospace;background:#22c55e;color:#052e16;pointer-events:none;white-space:nowrap;max-width:100%;overflow:hidden;text-overflow:ellipsis}
html.mor-xray-on [data-field-path]{outline:2px dashed #3b82f6!important;outline-offset:2px;background:rgba(59,130,246,.08)!important;position:relative;cursor:text}
html.mor-xray-on [data-field-path]::after{content:attr(data-xray-hint);position:absolute;bottom:calc(100% + 4px);left:0;z-index:10002;padding:2px 6px;border-radius:3px;font:10px/1.3 ui-monospace,monospace;background:#1d4ed8;color:#eff6ff;pointer-events:none;white-space:nowrap;opacity:0;transition:opacity .12s ease}
html.mor-xray-on [data-field-path]:hover::after{opacity:1}
html.mor-xray-on [data-edit-target^="icons."]{outline:2px solid #f97316!important;outline-offset:2px;box-shadow:0 0 0 3px rgba(249,115,22,.18)!important;position:relative}
html.mor-xray-on [data-edit-target^="icons."]::after{content:attr(data-xray-hint);position:absolute;bottom:calc(100% + 4px);left:0;z-index:10002;padding:2px 6px;border-radius:3px;font:10px/1.3 ui-monospace,monospace;background:#c2410c;color:#fff7ed;pointer-events:none;white-space:nowrap;opacity:0;transition:opacity .12s ease}
html.mor-xray-on [data-edit-target^="icons."]:hover::after{opacity:1}
html.mor-xray-on [data-edit-target]:not([data-field-path]):not([data-block-id]):not([data-edit-target^="icons."]){outline:2px dotted #a855f7!important;outline-offset:2px;background:rgba(168,85,247,.06)!important;position:relative}
html.mor-xray-on [data-edit-target]:not([data-field-path]):not([data-block-id]):not([data-edit-target^="icons."])::after{content:attr(data-xray-hint);position:absolute;bottom:calc(100% + 4px);left:0;z-index:10002;padding:2px 6px;border-radius:3px;font:10px/1.3 ui-monospace,monospace;background:#7e22ce;color:#faf5ff;pointer-events:none;white-space:nowrap;max-width:min(280px,90vw);overflow:hidden;text-overflow:ellipsis;opacity:0;transition:opacity .12s ease}
html.mor-xray-on [data-edit-target]:not([data-field-path]):not([data-block-id]):not([data-edit-target^="icons."]):hover::after{opacity:1}
html.mor-xray-on .mor-xray-hover{outline:1px solid rgba(40,149,240,.85)!important;outline-offset:1px;cursor:pointer}
/* Webflow-like selection chrome */
html.mor-xray-on .mor-canvas-selected{outline:2px solid #2895f0!important;outline-offset:2px!important;position:relative!important;z-index:50}
html.mor-xray-on .mor-canvas-selected::before{content:attr(data-mor-sel-label);position:absolute;top:-22px;left:-2px;z-index:100050;padding:3px 7px;border-radius:2px 2px 0 0;font:600 11px/1.2 system-ui,-apple-system,sans-serif;letter-spacing:.02em;background:#2895f0;color:#fff;pointer-events:none;white-space:nowrap;box-shadow:0 1px 3px rgba(0,0,0,.25)}
html.mor-xray-on .mor-canvas-editing{outline:2px solid #146ef5!important;outline-offset:2px!important;caret-color:#146ef5;min-width:1ch}
html.mor-xray-on .mor-canvas-editing::before{content:attr(data-mor-sel-label) " · editing";background:#146ef5}`;

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
                                            s.textContent = `html:not(.mor-xray-on) [data-field-path]:hover,[data-mor-edit]:hover{outline:2px dashed #3b82f6;cursor:text} [data-block-id]{cursor:grab;position:relative} .dragging{opacity:0.5} .drag-over{border-top:4px solid #3b82f6}`;
                                            doc.head.appendChild(s);
                                            doc.querySelectorAll('[data-block-id]').forEach(el => el.draggable = true);

                                            const TEXT_EDIT = 'h1,h2,h3,h4,h5,h6,p,span,a,li,label,button,td,th,figcaption,blockquote,strong,em,small';
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
                                            // Webflow-like: double-click any text node to edit in place
                                            doc.addEventListener('dblclick', e => {
                                                if (window.__morEditActive === false) return;
                                                let el = e.target.closest('[data-field-path],[data-mor-edit]');
                                                if (!el) el = e.target.closest(TEXT_EDIT);
                                                if (!el || el.closest('script,style,svg,code,pre,input,textarea,select')) return;
                                                e.preventDefault();
                                                e.stopPropagation();
                                                selectEl(doc, el);
                                                el.setAttribute('data-mor-edit-before', el.innerText);
                                                el.contentEditable = 'true';
                                                el.classList.add('mor-canvas-editing');
                                                el.focus();
                                                try {
                                                    const r = doc.createRange();
                                                    r.selectNodeContents(el);
                                                    const s = doc.defaultView.getSelection();
                                                    s.removeAllRanges();
                                                    s.addRange(r);
                                                } catch (_) {}
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
                                                const el = e.target.closest('[contenteditable="true"],[contenteditable=true]');
                                                if (!el || el.contentEditable !== "true") return;
                                                el.contentEditable = false;
                                                el.classList.remove('mor-canvas-editing');
                                                const before = el.getAttribute('data-mor-edit-before') || '';
                                                el.removeAttribute('data-mor-edit-before');
                                                const after = el.innerText;
                                                const path = el.getAttribute('data-field-path') || el.getAttribute('data-mor-edit');
                                                if (path) {
                                                    dioxus.send({action: "UPDATE_VALUE", target: path, value: after});
                                                } else if (before !== after) {
                                                    dioxus.send({
                                                        action: "PAGE_TEXT_EDIT",
                                                        old_text: before,
                                                        new_text: after,
                                                        tag: (el.tagName || '').toLowerCase()
                                                    });
                                                }
                                            }, true);
                                            // Enter commits the edit (Esc cancels)
                                            doc.addEventListener('keydown', e => {
                                                const el = doc.activeElement;
                                                if (!el || el.contentEditable !== 'true') return;
                                                if (e.key === 'Escape') {
                                                    e.preventDefault();
                                                    const before = el.getAttribute('data-mor-edit-before');
                                                    if (before != null) el.innerText = before;
                                                    el.contentEditable = false;
                                                    el.classList.remove('mor-canvas-editing');
                                                    el.removeAttribute('data-mor-edit-before');
                                                } else if (e.key === 'Enter' && !e.shiftKey && !/^(H[1-6]|P|LI|DIV|BLOCKQUOTE)$/i.test(el.tagName)) {
                                                    e.preventDefault();
                                                    el.blur();
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
                                                            dioxus.send({
                                                                action: "SELECT_DOM",
                                                                tag: pick.tagName.toLowerCase(),
                                                                classes: (cls && cls.baseVal !== undefined ? cls.baseVal : cls) || '',
                                                                label: selLabel(pick),
                                                                href: pick.getAttribute && pick.getAttribute('href') || '',
                                                                text: (labelEl && labelEl.innerText)
                                                                    || (pick.innerText || '').trim().slice(0, 80),
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
                                                    let href = json
                                                        .get("href")
                                                        .and_then(|h| h.as_str())
                                                        .filter(|s| !s.is_empty())
                                                        .map(str::to_string);
                                                    let text = json
                                                        .get("text")
                                                        .and_then(|t| t.as_str())
                                                        .filter(|s| !s.is_empty())
                                                        .map(str::to_string);
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
                                                        href,
                                                        text,
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
