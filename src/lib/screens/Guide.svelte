<script lang="ts">
  import Action from '#lib/components/Action.svelte';
  import Art from '#lib/components/Art.svelte';
  import FAQ from '#lib/components/FAQ.svelte';
  import Footer from '#lib/components/Footer.svelte';
  import { enter } from '#lib/motion.ts';
  let { privacy = false }: { privacy?: boolean } = $props();
  const walkthrough = [
    [
      'Make your room',
      'Give the room a name, choose 1–100 claim links, and set a base amount. Split it equally, or choose tiers: first place gets 3×, second and third get 2×.',
    ],
    [
      'Choose the small details',
      'Set when links expire and whether recipients can choose gift cards. Your pool ticket updates as you change the room, so you can check the total before continuing.',
    ],
    [
      'Fund once, share the links',
      'In this demo, confirm mock funding to generate your sheet. Print every claim, download a vector PDF, or copy the open links. Hand each ticket to one recipient.',
    ],
    [
      'Let them choose',
      'Recipients open a link, choose a shielded wallet or a gift card, and complete the claim. The demo scanner supplies a sample address if you want to try the entire wallet flow.',
    ],
    [
      'Watch the room, then wrap up',
      'The live room shows claimed links and the remaining pool. Close the room when you’re done to invalidate unused links and see the mock refund. Expired links also become unavailable.',
    ],
  ];
</script>

<svelte:head><title>{privacy ? 'Privacy, plainly' : 'How it works'} — Zool</title></svelte:head>
<main id="main" class="workspace guide-page" use:enter>
  <div class="guide-heading">
    <div>
      <p class="hand intro">
        {privacy ? 'no fine print gymnastics.' : 'from one pool to a room full of people.'}
      </p>
      <h1>
        {#if privacy}LESS TO COLLECT.<br /><mark>LESS TO EXPOSE.</mark>{:else}A LITTLE POOL.<br
          /><mark>A LOT LESS ADMIN.</mark>{/if}
      </h1>
    </div>
    <Art kind={privacy ? 'shield' : 'gift'} />
  </div>
  {#if privacy}<div class="guide-body privacy-guide">
      <section>
        <h2>THE SPREADSHEET NEVER HAPPENS.</h2>
        <p>
          Zool’s intended flow keeps the organizer from collecting a list of people and wallet
          addresses. Organizers hand out claim links; recipients choose a destination themselves.
          The dashboard only needs progress and amounts.
        </p>
      </section>
      <section>
        <h2>WHAT THIS DEMO SAVES.</h2>
        <p>
          Room settings, its funding flag, and which numbered links were claimed are stored in this
          browser’s localStorage. It also records whether a claim used the gift-card option. The
          demo does not request names or email addresses.
        </p>
        <p>
          Sample claim flags are stored separately for the current browser session. Trying a sample
          never changes your room. The recipient’s pasted wallet address is held temporarily in the
          page and cleared after the mock payout. It is not written to localStorage. There is no
          analytics or backend service in this implementation.
        </p>
      </section>
      <section>
        <h2>A LINK IS A LITTLE BEARER TICKET.</h2>
        <p>
          Anyone with an unused link can open it. Give each link to one recipient, keep it out of
          public chats, and choose an expiry appropriate to your room. In the demo, spent links
          can’t be redeemed again in the same browser state.
        </p>
      </section>
      <section>
        <h2>WHAT’S STILL TO COME.</h2>
        <p>
          Real shielding, network transactions, durable one-time redemption across devices, wallet
          scanning, and gift-card fulfillment require the backend and provider integrations. Local
          demo state is not a secure financial ledger.
        </p>
      </section>
      <section>
        <h2>YOUR BROWSER, YOUR DEMO DATA.</h2>
        <p>
          You can remove the demo room using your browser’s site-data controls. Creating and funding
          another room replaces the current one. The application currently supports one room per
          browser.
        </p>
      </section>
    </div>{:else}<ol class="walkthrough">
      {#each walkthrough as [title, copy], index}<li>
          <span class="walkthrough-number">0{index + 1}</span>
          <div>
            <h2>{title}</h2>
            <p>{copy}</p>
          </div>
        </li>{/each}
    </ol>
    <section class="guide-faq">
      <h2>STILL WONDERING?</h2>
      <FAQ />
    </section>{/if}
  <div class="guide-cta">
    <p class="hand">
      {privacy ? 'less paperwork is a pretty good start.' : 'the best way to get it? give it a go.'}
    </p>
    <Action href="/room/new">Set up a room</Action><Action href="/claim/demo" secondary cap={false}
      >Try a sample claim</Action
    >
  </div>
</main>
<Footer />
