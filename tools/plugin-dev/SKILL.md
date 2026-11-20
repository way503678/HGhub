---
name: hghub-plugin-dev
description: Develop and validate standalone JavaScript content-source or resolver plugins for HGhub HPP. Use for capabilities, host API integration, manifests and plugin debugging, not for changing the application UI.
---

# HGhub plugin development

Read references/status.md first: the public protocol is a design contract while the development host still has gaps. Read references/quickstart.md for workflow and references/protocol/README.md for the protocol map. Use assets/protocol.ts for shapes and assets/plugin-hello.js as a fictional example.

Use the site and output directory from the user's request. Inspect actual pages and requests before selecting capabilities. Keep site URLs, transformations and credentials inside the plugin, not the host. Never embed observed user session credentials in distributed source.

Deliver one self-contained UTF-8 ES module with meta and a default factory. Only declare capabilities the site supports. Use hg.http for APIs, resource objects for images/media and hg.storage for state needed after runtime reload. Factories construct objects and read settings without I/O.

Preserve upstream media, playlist, episode, category and server IDs. List position is not identity. Encode pagination state in returned cursors, not closure state. Respect ctx cancellation and do not schedule background work.

Check features or method presence for optional behavior. Put indispensable APIs in meta.requires. RSA encryption is implemented in the native host (1024–8192-bit SPKI/PKCS#1 PEM; PKCS1/OAEP-SHA1/OAEP-SHA256); declare crypto.rsaEncrypt in meta.requires when indispensable. The in-memory mock still requires an injected crypto adapter. Do not claim WebView, DASH, download or platform playback support from a type declaration alone.

For WebView login, offer a formless auth method when `hg.env.features.includes("webview")`. Open the view from the user-initiated `auth.submit` and set `doneWhen` for the site’s success URL or cookie names. `hg.webview.open` returns only `finalUrl`. Use a same-origin account endpoint to identify the user and implement `auth.getProfile`. The host manages login cookies per plugin and HTTPS origin. Use `hg.cookies.clear({ url })` on logout. `hg.cookies.get/set` access ordinary cookies.

Use hg.env.media and the supplied playback params.codecs; the client probes its WebView playback paths and reports a conservative codec intersection, which may be empty. Capability API results do not guarantee every profile or device playback. Headless/mock tests must inject media capabilities explicitly.

Map user-facing failures to hg.PluginError; distinguish upstream rejection from changed successful response shapes. Keep logs free of credentials and signed URLs.

Return subtitles with their actual format, language, label, default and resource headers. The host converts declared SRT to UTF-8 VTT even when the server uses application/octet-stream. Encoding uses BOM, then response charset, otherwise UTF-8; unmarked legacy encodings are not guessed. ASS rendering remains unsupported. Verify subtitle selection on the target device separately from proxy tests.

Use kind: external only for links handed to the system after user confirmation. HTTP(S), magnet, ed2k, rtsp and rtmp are allowed; app/mime are hints, not executable selection. No host headers, cookies or transformations are forwarded. Preloading never opens another application. Test installed-handler and missing-handler behavior on the device.

Read references/testing.md and references/distribution.md before delivery. Verify file import, manifest identity/version, selected install, repeat import, detail and playback. Distinguish synthetic tests, real QuickJS, upstream requests and native device playback evidence. Report unverified behavior and retain reproduction steps.

Follow publication constraints. Creating a plugin or manifest does not authorize uploading it. Public examples use fictional or user-controlled services.
