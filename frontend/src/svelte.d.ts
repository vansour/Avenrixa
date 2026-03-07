declare module '*.svelte' {
  import { SvelteComponentTyped } from 'svelte'

  interface SvelteComponentProps {
    [key: string]: any
  }

  export default class SvelteComponent extends SvelteComponentTyped<SvelteComponentProps> {
    constructor(options: {
      target: Element | DocumentFragment
      props?: SvelteComponentProps
      $$inline?: boolean
    })
  }
}
