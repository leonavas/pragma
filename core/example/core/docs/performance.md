# Performance

Fast by default: prebuilt HTML, `load` fetch starts in `<head>`, only the JS a page uses, link
prefetch, hashed assets, CSS with only used classes, budgets enforced by `check`.

- Navigate with `<a href>`.
- Shape skeletons like the final content.
- `<img>`: always `width` and `height`; `loading="lazy"` below the fold; AVIF/WebP in `public/`.
- `<for>` with a stable `key`.
- A page without interaction has no `.js`.
- Heavy code used by one page stays in that page; if it runs only after an interaction, `await import()` it inside the action.
- `load` returns only what the page shows. Run independent calls with `Promise.all`.
- Never raise a budget in `pragma.json`: shrink the code.
- Never import large modules in `lib/state/`, fetch data in `mount`, or enable `ssr` by default.
