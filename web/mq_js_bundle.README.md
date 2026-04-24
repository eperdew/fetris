# mq_js_bundle.js

Downloaded from the macroquad CDN, then patched to add missing WebGL functions.

## Regenerating

```
curl -o mq_js_bundle.js https://not-fl3.github.io/miniquad-samples/mq_js_bundle.js
```

Then re-apply the patch at the bottom of the file: a `miniquad_add_plugin` block that
adds `glCheckFramebufferStatus`, `glBlitFramebuffer`, `glDeleteRenderbuffers`,
`glFramebufferRenderbuffer`, `glReadBuffer`, and `glRenderbufferStorageMultisample`.

These functions exist in miniquad 0.4.8's `gl.js` but were missing from the CDN bundle,
causing a wasm trap when `render_target_ex` called `glCheckFramebufferStatus` and
asserted the result was nonzero.

The patch block lives at the bottom of `mq_js_bundle.js` — copy it before overwriting.
