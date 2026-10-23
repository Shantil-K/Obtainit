import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// @material/web elements are focusable, keyboard-accessible controls, so
// Svelte's "click on a non-interactive element" checks don't apply to them.
const mdA11y = /`<md-[a-z-]+>`/;

export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    warningFilter: (w) => !(w.code.startsWith("a11y") && mdA11y.test(w.message)),
  },
};
