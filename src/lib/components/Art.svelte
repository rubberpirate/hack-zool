<script lang="ts">
  let {
    kind = 'envelope',
    class: className = '',
  }: {
    kind?: 'envelope' | 'coin' | 'coin-check' | 'gift' | 'shield' | 'success' | 'cloud' | 'badge';
    class?: string;
  } = $props();
</script>

<svg
  class="art {className}"
  viewBox={kind === 'success' ? '0 0 260 260' : kind === 'envelope' ? '0 0 300 260' : '0 0 160 160'}
  fill="none"
  xmlns="http://www.w3.org/2000/svg"
  aria-hidden="true"
>
  {#if kind === 'coin'}
    <circle
      cx="80"
      cy="80"
      r="69"
      fill="var(--yellow)"
      stroke="currentColor"
      stroke-width="5"
    /><circle
      cx="80"
      cy="80"
      r="54"
      stroke="currentColor"
      stroke-width="3"
      stroke-dasharray="8 6"
    /><path
      d="m57 56 45-5-46 57 47-5"
      stroke="currentColor"
      stroke-width="5"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  {:else if kind === 'coin-check'}
    <circle
      cx="76"
      cy="82"
      r="62"
      fill="var(--yellow)"
      stroke="currentColor"
      stroke-width="5"
    /><circle
      cx="76"
      cy="82"
      r="48"
      stroke="currentColor"
      stroke-width="3"
      stroke-dasharray="7 5"
    /><path
      d="m54 62 38-4-40 48 40-4"
      stroke="currentColor"
      stroke-width="4.5"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
    <!-- Verified tick badge on coin -->
    <circle
      cx="116"
      cy="44"
      r="32"
      fill="var(--mint)"
      stroke="black"
      stroke-width="4"
    />
    <path
      d="m105 45 7 7 16-16"
      stroke="black"
      stroke-width="4.5"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  {:else if kind === 'shield'}
    <path
      d="m80 12 58 18 2 63c-5 28-32 43-60 55-29-12-53-27-58-55l-2-63z"
      fill="var(--mint)"
      stroke="currentColor"
      stroke-width="4"
    /><path
      d="M52 68q8-11 16 0m25 0q8-11 16 0M58 94q22 23 45-2"
      stroke="currentColor"
      stroke-width="4"
      stroke-linecap="round"
    />
  {:else if kind === 'gift'}
    <path
      d="M39 45Q5 10 39 13q24 0 40 32 22-38 42-31 30 12-17 32"
      stroke="currentColor"
      stroke-width="4"
      stroke-linecap="round"
    /><rect
      x="16"
      y="44"
      width="129"
      height="99"
      rx="15"
      fill="var(--accent)"
      stroke="currentColor"
      stroke-width="4"
    /><path d="M16 83h129" stroke="currentColor" stroke-width="4" />
  {:else if kind === 'cloud'}
    <path
      d="M29 69C20 45 49 22 69 40c28-30 61-7 59 17 34-1 40 42 9 45H26C0 102 4 72 29 69Z"
      fill="var(--yellow)"
      stroke="currentColor"
      stroke-width="4"
    /><path
      d="M61 76v6m30-6v6m-30 8q15 14 30 0"
      stroke="currentColor"
      stroke-width="3"
      stroke-linecap="round"
    />
  {:else if kind === 'badge'}
    <circle cx="80" cy="80" r="74" fill="var(--accent)" stroke="currentColor" stroke-width="3" />
    <path id="badge-ring" d="M80 20a60 60 0 1 1-.1 0" />
    <text
      fill="currentColor"
      font-family="sans-serif"
      font-size="12.5"
      font-weight="800"
      letter-spacing="1.3"
      ><textPath href="#badge-ring" startOffset="0"
        >NO WALLET LIST • NO WALLET LIST •
      </textPath></text
    >
    <circle cx="80" cy="80" r="22" fill="white" stroke="currentColor" stroke-width="2.5" /><path
      d="m74 72-2 6m14-4-2 6m-15 4q8 11 18 1"
      stroke="currentColor"
      stroke-width="3"
      stroke-linecap="round"
    />
  {:else if kind === 'envelope'}
    <g class="coin-in-envelope"
      ><circle
        cx="164"
        cy="83"
        r="50"
        fill="var(--yellow)"
        stroke="currentColor"
        stroke-width="4"
      /><circle
        cx="164"
        cy="83"
        r="38"
        stroke="currentColor"
        stroke-width="3"
        stroke-dasharray="6 5"
      /><path
        d="m148 66 29-4-30 39 30-4"
        stroke="currentColor"
        stroke-width="4"
        stroke-linecap="round"
        stroke-linejoin="round"
      /></g
    >
    <rect
      x="39"
      y="116"
      width="222"
      height="143"
      rx="20"
      fill="white"
      stroke="currentColor"
      stroke-width="4"
    />
    <path
      d="m43 127 108 79 107-79M125 223q25 20 47-1M125 187v6m46-6v6M22 65v18m-9-9h18"
      stroke="currentColor"
      stroke-width="4"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  {:else if kind === 'success'}
    <g class="success-coin"
      ><circle
        cx="130"
        cy="49"
        r="43"
        fill="var(--yellow)"
        stroke="currentColor"
        stroke-width="5"
      /><circle
        cx="130"
        cy="49"
        r="33"
        stroke="currentColor"
        stroke-width="3"
        stroke-dasharray="6 4"
      /><path
        d="m117 32 27-3-29 37 29-4"
        stroke="currentColor"
        stroke-width="4"
        stroke-linecap="round"
      /></g
    >
    <g stroke="currentColor" stroke-width="4.5" stroke-linecap="round" stroke-linejoin="round"
      ><circle cx="130" cy="127" r="16" fill="white" /><path
        d="m124 130 5 4 7-6m-6 15v49m0-42-44-40m44 40 44-40m-44 80-27 46m27-46 28 46M33 79v21m-10-10h21m179-28v18m-9-9h18M47 167v14m-7-7h14M18 250q12-13 25 0t26 0 26 0 26 0 26 0 26 0 26 0 26 0 26 0"
      /></g
    >
  {/if}
</svg>
