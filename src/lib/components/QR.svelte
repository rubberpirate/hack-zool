<script lang="ts">
  import { onMount } from 'svelte';
  let { value, label = 'Scan this one-time claim link' }: { value: string; label?: string } =
    $props();

  let qrData = $state<{
    path: string;
    viewBox: string;
    center: number;
    badgeSize: number;
  } | null>(null);
  let failed = $state(false);

  onMount(() => {
    let active = true;
    import('qrcode')
      .then(async ({ default: qr }) => {
        // High error correction level (H = 30% redundancy) ensures complete scannability
        // with the native central Z-Split logo
        const code = qr.create(value, {
          errorCorrectionLevel: 'H',
        });
        const size = code.modules.size;
        const margin = 1;
        const viewBoxSize = size + margin * 2;
        const center = viewBoxSize / 2;
        // Central badge clearance radius in module units
        const badgeRadius = Math.max(3.6, size * 0.145);
        const badgeSize = badgeRadius * 2;

        let path = '';
        for (let r = 0; r < size; r++) {
          for (let c = 0; c < size; c++) {
            if (code.modules.get(r, c)) {
              const x = c + margin + 0.5;
              const y = r + margin + 0.5;
              const dist = Math.hypot(x - center, y - center);
              // Clear center module positions so QR modules do not clash with the central brand badge
              if (dist > badgeRadius) {
                path += `M${c + margin} ${r + margin}h1v1h-1z `;
              }
            }
          }
        }

        if (active) {
          qrData = {
            path,
            viewBox: `0 0 ${viewBoxSize} ${viewBoxSize}`,
            center,
            badgeSize,
          };
        }
      })
      .catch(() => (failed = true));

    return () => {
      active = false;
    };
  });
</script>

<div class="qr-image native-qr" role="img" aria-label={label}>
  {#if qrData}
    <svg
      viewBox={qrData.viewBox}
      xmlns="http://www.w3.org/2000/svg"
      shape-rendering="crispEdges"
      aria-hidden="true"
    >
      <!-- Background -->
      <rect width="100%" height="100%" fill="white" />
      <!-- Data modules -->
      <path d={qrData.path} fill="black" />

      <!-- Native Z-Split Center Brand Mark -->
      <g
        transform="translate({qrData.center - qrData.badgeSize / 2}, {qrData.center - qrData.badgeSize / 2})"
        shape-rendering="geometricPrecision"
      >
        <!-- Badge Container with Subtle Border -->
        <rect
          x="0"
          y="0"
          width={qrData.badgeSize}
          height={qrData.badgeSize}
          rx={qrData.badgeSize * 0.28}
          fill="white"
          stroke="black"
          stroke-width={qrData.badgeSize * 0.07}
        />
        <!-- Inner Tag Shape / ZEC Shield Motif -->
        <g transform="scale({qrData.badgeSize / 60}) translate(6, 6)">
          <path
            d="M11 2h26q9 0 9 10v22q0 10-10 10H27L22 49l-5-5H11Q2 44 2 34V12Q2 2 11 2Z"
            fill="black"
          />
          <!-- The crisp Z logo -->
          <path d="M14 11h15v5L19 31h10v5H13v-5l10-15H14z" fill="white" />
          <!-- The split slash/accent line -->
          <path d="M35 11v19m0 4v3" stroke="var(--accent, #C3B8EE)" stroke-width="4" stroke-linecap="round" />
        </g>
      </g>
    </svg>
  {:else if failed}
    <span class="qr-fallback">Open claim ↗</span>
  {:else}
    <span class="qr-placeholder"></span>
  {/if}
</div>

<style>
  .native-qr {
    position: relative;
    user-select: none;
    background: white;
    border-radius: 0.5rem;
    overflow: hidden;
  }

  .native-qr svg {
    width: 100%;
    height: 100%;
    display: block;
    animation: qr-fade-in var(--dur-base, 250ms) var(--ease-out, cubic-bezier(0.22, 1, 0.36, 1));
  }
</style>
