<script lang="ts">
  import Action from '#lib/components/Action.svelte';
  import Art from '#lib/components/Art.svelte';
  import FallingCoins from '#lib/components/FallingCoins.svelte';
  import Footer from '#lib/components/Footer.svelte';
  import FAQ from '#lib/components/FAQ.svelte';
  import Icon from '#lib/components/Icon.svelte';
  import { enter, reveal } from '#lib/motion.ts';
  const steps = [
    {
      number: '01',
      title: 'MAKE A LITTLE POOL.',
      copy: 'Name your room, choose how many people you’re paying, and set an amount. Keep it equal or give the podium a little extra.',
      art: 'coin' as const,
      detail: '50 claims × 0.50 ZEC',
      note: 'one deposit. a whole room paid.',
    },
    {
      number: '02',
      title: 'HAND OUT THE GOOD STUFF.',
      copy: 'Your pool becomes one-time claim links. Print a QR sheet, copy the links, or hand out tickets in person. No names. No address collection.',
      art: 'envelope' as const,
      detail: 'One link. One person. One claim.',
      note: 'a little envelope with their name off it.',
    },
    {
      number: '03',
      title: 'THEY TAKE IT FROM HERE.',
      copy: 'Recipients scan, pick a shielded wallet or a gift card, and claim their share. You see the room’s progress. Their destination stays out of your dashboard.',
      art: 'shield' as const,
      detail: 'Scan. Pick. Claim. Done.',
      note: 'less admin for you. more choice for them.',
    },
  ];
</script>

<main id="main" class="marketing-page">
  <FallingCoins />
  <section class="landing" aria-label="Pay the room" use:enter>
    <div class="hero-intro hand">shielded ZEC payouts, hand-delivered</div>
    <div class="hero-heading-wrap">
      <h1>PAY 50 PEOPLE.<br />COLLECT <span class="zero">0</span> ADDRESSES.</h1>
    </div>
    <p class="hero-copy hand">
      One pool of ZEC. A handful of one-time claim links.<br class="desktop-break" /> Everyone gets their
      share. Nobody fills in a spreadsheet.
    </p>
    <div class="hero-actions">
      <Action href="/room/new">Fund a Z-Pool</Action><Action href="/claim" secondary cap={false}
        >Claim prize</Action
      >
    </div>
    <div class="hero-smallprint">
      <span><Icon name="check" size={14} /> No address collection</span><span
        ><Icon name="check" size={14} /> No recipient account</span
      >
    </div>
    <a class="scroll-cue" href="#how-it-works"
      ><span class="hand">a little less admin. a lot more privacy.</span><Icon
        name="right"
        size={19}
      /></a
    >
  </section>
  <section id="how-it-works" class="product-story">
    <div class="story-opening" use:reveal>
      <p class="hand">for the people who bring people together.</p>
      <h2>PAYING PEOPLE SHOULDN’T<br />BECOME <mark>A WHOLE JOB.</mark></h2>
      <p>
        You organized the hackathon. Ran the event. Brought the contributors together. Now pay them
        without chasing fifty wallet addresses.
      </p>
    </div>
    <div class="story-steps">
      {#each steps as step}<article class="story-step" use:reveal>
          <div class="step-text">
            <span class="step-number">{step.number}</span>
            <h3>{step.title}</h3>
            <p>{step.copy}</p>
            <span class="hand step-note">{step.note}</span>
          </div>
          <div class="step-visual">
            <Art kind={step.art} /><span class="story-ticket-label">{step.detail}</span><svg
              class="story-arrow"
              viewBox="0 0 110 70"
              aria-hidden="true"
              ><path
                d="M4 9q24 59 95 27m-23-8 24 7-12 20"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              /></svg
            >
          </div>
        </article>{/each}
    </div>
  </section>
  <section class="payout-story marketing-section" use:reveal>
    <div class="section-heading">
      <p class="hand">their share. their call.</p>
      <h2>TWO WAYS TO<br /><mark>MAKE THEIR DAY.</mark></h2>
      <p>
        A crypto regular and a first-timer can be in the same room. The payout should work for both.
      </p>
    </div>
    <div class="payout-comparison">
      <a href="/claim/demo/payout" class="payout-feature wallet-feature"
        ><Art kind="shield" />
        <div>
          <h3>STRAIGHT TO<br />THEIR WALLET.</h3>
          <p>
            A shielded ZEC payout to the address they choose. Entered in the claim flow, kept out of
            your room dashboard.
          </p>
          <span class="feature-link">TRY THE WALLET FLOW <Icon name="right" size={20} /></span>
        </div></a
      ><a href="/claim/demo/payout?method=gift" class="payout-feature gift-feature"
        ><Art kind="gift" />
        <div>
          <h3>OR A LITTLE<br />SOMETHING NICE.</h3>
          <p>
            No wallet? They can pick a gift card instead. Turn on the option when you set up the
            room and let each person decide.
          </p>
          <span class="feature-link">TRY THE GIFT CARD FLOW <Icon name="right" size={20} /></span>
        </div></a
      >
    </div>
  </section>
  <section class="privacy-story marketing-section" use:reveal>
    <div class="privacy-drawing">
      <Art kind="shield" /><span class="hand">the spreadsheet<br />never happens.</span>
    </div>
    <div class="section-heading">
      <p class="hand">privacy starts before the transaction.</p>
      <h2>YOU SEE PROGRESS.<br /><mark>NOT PEOPLE’S BUSINESS.</mark></h2>
      <p>
        See how many links have been claimed, how much is left, and when the room expires. You don’t
        need a list connecting people to wallet addresses.
      </p>
      <ul>
        <li><Icon name="check" size={18} /> One-time links, handed out by you</li>
        <li><Icon name="check" size={18} /> Wallet destinations stay off the dashboard</li>
        <li><Icon name="check" size={18} /> Unused links expire on your schedule</li>
      </ul>
      <a class="inline-link" href="/privacy"
        >Privacy, in plain language <Icon name="right" size={18} /></a
      >
    </div>
  </section>
  <section class="occasions marketing-section" use:reveal>
    <p class="hand">same room. all sorts of reasons.</p>
    <h2>FOR THE PEOPLE<br />WHO <mark>SHOWED UP.</mark></h2>
    <div class="occasion-list">
      <span>Hackathon prizes</span><span>Builder bounties</span><span>Contributor rewards</span
      ><span>Community events</span><span>Grants & giveaways</span>
    </div>
  </section>
  <section class="faq-section marketing-section" use:reveal>
    <div class="section-heading">
      <p class="hand">a few things you might be wondering.</p>
      <h2>GOOD<br /><mark>QUESTIONS.</mark></h2>
      <a class="inline-link" href="/how-it-works"
        >The full walkthrough <Icon name="right" size={18} /></a
      >
    </div>
    <FAQ />
  </section>
  <section class="closing-cta" use:reveal>
    <Art kind="cloud" />
    <p class="hand">you bring the people. we’ll bring the links.</p>
    <h2>GO ON.<br />MAKE SOMEONE’S DAY.</h2>
    <Action href="/room/new">Set up your room</Action>
    <p class="closing-demo">Try the complete demo. No wallet connection or real funds required.</p>
  </section>
</main>
<Footer />
