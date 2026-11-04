<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import { app } from '#lib/state.svelte.ts';
  let {
    children,
    href,
    onclick,
    secondary = false,
    cap = true,
    icon = 'plus',
    disabled = false,
    type = 'button',
    class: className = '',
  }: {
    children: Snippet;
    href?: string;
    onclick?: (e: MouseEvent) => void;
    secondary?: boolean;
    cap?: boolean;
    icon?: string;
    disabled?: boolean;
    type?: 'button' | 'submit';
    class?: string;
  } = $props();
</script>

{#snippet content()}
  <span class="action-label">{@render children()}</span>{#if cap}<span class="action-cap"
      ><Icon name={icon} size={23} /></span
    >{/if}
{/snippet}
{#if href}<a
    {href}
    class="action {secondary ? 'secondary' : 'primary'} {cap ? 'has-cap' : ''} {className}"
    {onclick}>{@render content()}</a
  >
{:else}<button
    {type}
    disabled={disabled || !app.ready}
    {onclick}
    class="action {secondary ? 'secondary' : 'primary'} {cap ? 'has-cap' : ''} {className}"
    >{@render content()}</button
  >{/if}
