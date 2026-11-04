<script lang="ts">
  const questions = [
    [
      'Do I need everyone’s wallet address?',
      'No. You create one-time claim links and hand them out. Each recipient chooses their own payout destination in the claim flow. There is no address spreadsheet for you to manage.',
    ],
    [
      'What if someone doesn’t have a wallet?',
      'Enable gift cards when setting up the room. The recipient can choose a gift card instead of a wallet payout. The current demo includes sample Amazon, Steam, and Starbucks options.',
    ],
    [
      'Can the same QR code be claimed twice?',
      'The demo marks a link as spent after its first redemption. Anyone holding an unused link can open it, so hand each link to one person and keep it private. Secure, cross-device redemption will need the backend.',
    ],
    [
      'What happens to unclaimed funds?',
      'You choose an expiry when creating the room. Expired links become unavailable, and closing the room shows a refund of the remaining demo pool. No real funds move in this frontend.',
    ],
    [
      'Is this connected to Zcash yet?',
      'This is a complete interactive frontend demo. Funding, wallet scanning, payouts, gift cards, and refunds are simulated. The wallet and network integrations will come with the backend.',
    ],
  ];
</script>

<div class="faq-list">
  {#each questions as [question, answer]}<details>
      <summary>{question}<span aria-hidden="true">+</span></summary>
      <p>{answer}</p>
    </details>{/each}
</div>
