import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import preprocessor from 'svelte-preprocess';

export default {
  preprocess: [
    vitePreprocess(),
    preprocessor(),
  ],
};
