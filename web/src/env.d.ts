/// <reference types="vite/client" />
/// <reference types="vite-plugin-pwa/vue" />

interface ImportMetaEnv {
  /** Link to the source repository, set by the deploy workflow. */
  readonly VITE_SOURCE_URL?: string;
}

/** The app version from package.json, filled in at build time. */
declare const __APP_VERSION__: string;
