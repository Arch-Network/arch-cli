interface ImportMetaEnv {
    readonly VITE_RPC_URL: string
    readonly VITE_BLOCKS_PER_PAGE: string
  }
  
  interface ImportMeta {
    readonly env: ImportMetaEnv
  }