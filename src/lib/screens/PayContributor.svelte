<script lang="ts">
  import {
    app,
    notify,
    copyText,
    addContributorNotification,
    getOrCreateRepoPool,
    deductFromRepoPool,
    depositToRepoPool,
  } from '#lib/state.svelte.ts';
  import {
    BITREFILL_CATALOG,
    purchaseBitrefillVoucherForPR,
    ZEC_PRICE_USD,
    type RepoPool,
  } from '#lib/bitrefill.ts';
  import Action from '#lib/components/Action.svelte';
  import Art from '#lib/components/Art.svelte';
  import Icon from '#lib/components/Icon.svelte';
  import { enter } from '#lib/motion.ts';

  // Interactive GitHub Bot Simulation State: Just GitHub Handle!
  let repo = $state('privacy-scaling-explorations/zk-pool');
  let prNumber = $state(42);
  let prTitle = $state('feat(snark): implement constant-time scalar multiplication verification');
  let contributorGithub = $state('alex-dev');
  let voucherBrand = $state('Amazon Gift Card');
  let voucherAmountUsd = $state(25);
  let bitrefillVoucherCode = $state('BITREFILL-AMZN-9824-ZEC');
  let catalogFilter = $state<'all' | 'in' | 'global'>('all');
  let depositAmount = $state(5);
  let showDepositModal = $state(false);

  // Active repository pool state
  const currentPool: RepoPool = $derived(getOrCreateRepoPool(repo));

  // Filtered Bitrefill products
  const filteredProducts = $derived(
    catalogFilter === 'all'
      ? BITREFILL_CATALOG
      : catalogFilter === 'in'
        ? BITREFILL_CATALOG.filter((p) => p.countries.includes('IN'))
        : BITREFILL_CATALOG.filter((p) => p.countries.includes('Global') || p.countries.includes('US'))
  );

  // Streamlined bot command without requiring any SMTP or email!
  const botCommand = $derived(
    `@zool reward @${contributorGithub.replace(/^@/, '')} $${voucherAmountUsd} ${voucherBrand}`
  );

  // Guardrail check
  const isOverPrLimit = $derived(voucherAmountUsd > currentPool.maxLimitPerPrUsd);
  const isOverPoolBalance = $derived(voucherAmountUsd > currentPool.poolBalanceUsd);

  // Bot workflow stages
  let step = $state<'idle' | 'commenting' | 'processing' | 'purchased' | 'sent'>('idle');
  let latestInvoiceId = $state<string>('inv_bitrefill_live_89234a71');
  let prComments = $state([
    {
      author: 'alex-dev',
      avatar: 'https://avatars.githubusercontent.com/u/1024025?v=4',
      badge: 'Contributor',
      time: '3 hours ago',
      body: 'I benchmarked the constant-time scalar mul on mobile devices; latency dropped by 34% with zero timing leak. PR is ready for review!',
    },
    {
      author: 'maintainer-satoshi',
      avatar: 'https://avatars.githubusercontent.com/u/583231?v=4',
      badge: 'Maintainer',
      time: '1 hour ago',
      body: 'Verified & merged! Outstanding contribution. Open-source maintainers often fail to reward contributors swiftly, so sending you a Bitrefill gift voucher right away.',
    },
  ]);

  async function triggerBotReward() {
    const handle = contributorGithub.replace(/^@/, '').trim() || 'alex-dev';

    // Verify guardrails before starting
    if (isOverPrLimit) {
      notify(`Cannot proceed: $${voucherAmountUsd} exceeds the repo's guardrail limit of $${currentPool.maxLimitPerPrUsd} per PR.`);
      return;
    }
    if (isOverPoolBalance) {
      notify(`Cannot proceed: Repo pool balance ($${currentPool.poolBalanceUsd.toFixed(2)}) is insufficient.`);
      return;
    }

    step = 'commenting';
    // Add maintainer command comment
    prComments = [
      ...prComments,
      {
        author: 'maintainer-satoshi',
        avatar: 'https://avatars.githubusercontent.com/u/583231?v=4',
        badge: 'Maintainer',
        time: 'Just now',
        body: botCommand,
      },
    ];

    step = 'processing';
    await new Promise((r) => setTimeout(r, 850));

    // Execute Bitrefill purchase loop
    const result = await purchaseBitrefillVoucherForPR({
      repoPool: currentPool,
      contributor: handle,
      brand: voucherBrand,
      amountUsd: voucherAmountUsd,
      prNumber,
    });

    if (!result.success || !result.invoice) {
      notify(result.error || 'Bitrefill purchase failed');
      step = 'idle';
      return;
    }

    step = 'purchased';
    await new Promise((r) => setTimeout(r, 900));

    // Deduct live from the repository pool & record invoice
    deductFromRepoPool(repo, voucherAmountUsd, result.invoice);
    bitrefillVoucherCode = result.invoice.voucherCode;
    latestInvoiceId = result.invoice.id;

    prComments = [
      ...prComments,
      {
        author: 'zool[bot]',
        avatar: 'https://avatars.githubusercontent.com/u/9919?v=4',
        badge: 'Bot • Verified',
        time: 'Just now',
        body: `🎉 **Bitrefill Voucher Dispatched!**\n\nPurchased **$${voucherAmountUsd} ${voucherBrand}** via Bitrefill agent rails funded from repository shielded pool.\n\n• **Contributor:** @${handle}\n• **Delivery:** In-App Notification Bar (Zero Email / Zero SMTP)\n• **Voucher Code:** \`${bitrefillVoucherCode}\`\n• **Bitrefill Invoice:** \`${latestInvoiceId}\`\n• **Repo Pool Balance:** $${currentPool.poolBalanceUsd.toFixed(2)} USD remaining`,
      },
    ];

    // Push directly to contributor's notification bar (Zero SMTP!)
    addContributorNotification({
      contributorGithub: handle,
      repo,
      prNumber,
      brand: voucherBrand,
      amountUsd: voucherAmountUsd,
      voucherCode: bitrefillVoucherCode,
      claimUrl: '/claim/sample-4',
      bitrefillInvoiceId: latestInvoiceId,
    });

    step = 'sent';
  }

  function handleDeposit() {
    if (depositAmount <= 0) return;
    depositToRepoPool(repo, depositAmount);
    showDepositModal = false;
  }

  function resetDemo() {
    step = 'idle';
    prComments = prComments.slice(0, 2);
  }
</script>

<svelte:head>
  <title>Pay Contributor (Bitrefill + Shielded Repo Pool) — Z-Split</title>
  <meta
    name="description"
    content="Reward open-source contributors directly from GitHub PR comments with instant Bitrefill vouchers funded by shielded repo pools."
  />
</svelte:head>

<main id="main" class="workspace github-workflow-page" use:enter>
  <!-- Hero Section -->
  <header class="gh-hero">
    <div class="gh-kicker-badge">
      <Icon name="github" size={16} />
      <span>AUTONOMOUS BITREFILL AGENT • LANE 1 GUARDRAILS & LANE 3 TRIGGERS</span>
    </div>
    <h1>REWARD CONTRIBUTORS.<br /><mark>DIRECTLY IN PRs.</mark></h1>
    <p class="hand gh-lead">
      In open-source repositories, contributors often go unrewarded due to cross-border fiat friction.
      Every repository gets a <strong>Shielded Zcash Pool</strong>. Maintainers summon <code>@zool</code> in PR comments;
      the bot purchases a <strong>Bitrefill Voucher</strong> autonomously and delivers it straight to the contributor's in-app notification bar.
    </p>
  </header>

  <div class="gh-workflow-grid">
    <!-- Left Column: Maintainer Configuration & Repo Pool -->
    <section class="gh-control-panel">
      <!-- Repo Pool Balance & Guardrails Status Card -->
      <div class="pool-status-card" use:enter>
        <div class="pool-status-header">
          <div>
            <span class="label">REPOSITORY SHIELDED POOL</span>
            <h3 style="font-size: 1.35rem; font-weight: 800; margin-top: 0.15rem;">{currentPool.name}</h3>
            <div class="pool-balance-metric">
              <span class="zec-val">{currentPool.poolBalanceZec.toFixed(2)} ZEC</span>
              <span class="usd-val">≈ ${currentPool.poolBalanceUsd.toFixed(2)} USD</span>
            </div>
          </div>
          <button
            type="button"
            class="pill mini-pill"
            style="border-color: var(--terracotta); color: var(--terracotta);"
            onclick={() => (showDepositModal = !showDepositModal)}
          >
            + Top Up Pool
          </button>
        </div>

        {#if showDepositModal}
          <div class="pool-fund-action" use:enter>
            <input
              type="number"
              min="1"
              max="100"
              step="1"
              bind:value={depositAmount}
              style="width: 5rem; padding: 0.4rem; border: 1.5px solid black; border-radius: 0.5rem; font-weight: 700;"
            />
            <span style="font-size: 0.85rem; font-weight: 700;">ZEC</span>
            <button type="button" class="pill mini-pill" onclick={handleDeposit}>Deposit Shielded Funds</button>
          </div>
        {/if}

        <!-- Guardrails Strip (Lane 1 Compliance) -->
        <div class="pool-guardrail-strip">
          <div class="guardrail-item">
            <span class="guardrail-label">GUARDRAIL: MAX PER PR</span>
            <span class="guardrail-val active-limit">${currentPool.maxLimitPerPrUsd} USD max</span>
          </div>
          <div class="guardrail-item">
            <span class="guardrail-label">TOTAL DISBURSED</span>
            <span class="guardrail-val">${currentPool.totalSpentUsd} USD ({currentPool.totalVouchersIssued} vouchers)</span>
          </div>
        </div>
      </div>

      <!-- Maintainer Control Card -->
      <div class="panel-card">
        <span class="label">MAINTAINER REWARD CONFIG</span>
        <h2>PR REWARD BOT</h2>
        <p class="hand">Summon the agent in GitHub to purchase Bitrefill products from the repo pool.</p>

        <div class="field">
          <label for="gh-repo">GITHUB REPOSITORY POOL</label>
          <div class="repo-input">
            <span class="repo-prefix">github.com/</span>
            <select
              id="gh-repo"
              bind:value={repo}
              style="border: none; background: transparent; width: 100%; font-size: 0.95rem; font-weight: 700; padding: 0.65rem 0.5rem;"
            >
              <option value="privacy-scaling-explorations/zk-pool">privacy-scaling-explorations/zk-pool</option>
              <option value="electric-coin-company/zcash">electric-coin-company/zcash</option>
              <option value="sveltejs/svelte">sveltejs/svelte</option>
            </select>
          </div>
        </div>

        <div class="field">
          <label for="gh-contributor">CONTRIBUTOR GITHUB HANDLE</label>
          <div class="handle-input">
            <span>@</span>
            <input id="gh-contributor" type="text" bind:value={contributorGithub} placeholder="github-username" />
          </div>
          <span class="field-hint hand">Voucher code delivers to @{contributorGithub.replace(/^@/, '')}'s in-app notification bar. Zero SMTP / Zero Email required!</span>
        </div>

        <div class="field-row">
          <div class="field">
            <div style="display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 0.25rem;">
              <label for="gh-brand" style="margin-bottom: 0;">BITREFILL PRODUCT</label>
              <div style="display: flex; gap: 0.3rem;">
                <button
                  type="button"
                  class="pill mini-pill"
                  style="font-size: 0.65rem; padding: 0.1rem 0.4rem;"
                  class:primary={catalogFilter === 'all'}
                  onclick={() => (catalogFilter = 'all')}
                >
                  All
                </button>
                <button
                  type="button"
                  class="pill mini-pill"
                  style="font-size: 0.65rem; padding: 0.1rem 0.4rem;"
                  class:primary={catalogFilter === 'in'}
                  onclick={() => (catalogFilter = 'in')}
                >
                  India 🇮🇳
                </button>
              </div>
            </div>
            <select id="gh-brand" bind:value={voucherBrand}>
              {#each filteredProducts as product}
                <option value={product.name}>{product.name} ({product.currency})</option>
              {/each}
            </select>
          </div>
          <div class="field">
            <label for="gh-usd">AMOUNT (USD)</label>
            <div class="amount-input">
              <input id="gh-usd" type="number" min="5" max="500" step="5" bind:value={voucherAmountUsd} />
              <span>USD</span>
            </div>
          </div>
        </div>

        {#if isOverPrLimit}
          <div class="guardrail-warning" style="background: #fff4f2; border: 1.5px solid var(--terracotta); border-radius: 0.6rem; padding: 0.65rem; margin-top: 0.5rem;" use:enter>
            <strong style="color: var(--terracotta); font-size: 0.8rem; display: block;">⚠️ Guardrail Threshold Exceeded</strong>
            <span style="font-size: 0.78rem; color: var(--ink);">Amount (${voucherAmountUsd}) exceeds maximum PR limit of ${currentPool.maxLimitPerPrUsd} without multi-maintainer approval.</span>
          </div>
        {/if}

        <!-- Generated Bot Command Box -->
        <div class="bot-cmd-box">
          <div class="cmd-header">
            <span class="label">GITHUB PR COMMENT COMMAND</span>
            <button type="button" class="mini-copy" onclick={() => copyText(botCommand, 'Bot command copied!')}>
              <Icon name="copy" size={14} /> Copy
            </button>
          </div>
          <code>{botCommand}</code>
        </div>

        <!-- Call Bot Button -->
        <div class="panel-action">
          {#if step === 'sent'}
            <div class="success-banner" use:enter>
              <span class="check-mini"><Icon name="check" size={16} /></span>
              <span>Delivered to notification bar! (Invoice: {latestInvoiceId})</span>
              <button type="button" class="pill mini-pill" onclick={resetDemo}>Test again</button>
            </div>
          {:else}
            <button
              type="button"
              class="action primary has-cap"
              disabled={step !== 'idle' || isOverPrLimit || isOverPoolBalance}
              onclick={triggerBotReward}
            >
              <span class="action-label">
                {step === 'processing'
                  ? 'Connecting to Bitrefill API…'
                  : step === 'purchased'
                    ? 'Settling from Shielded Pool…'
                    : 'Call Bot in PR & Send Voucher'}
              </span>
              <span class="action-cap"><Icon name="check" size={24} /></span>
            </button>
          {/if}
        </div>
      </div>

      <!-- Promises & Guardrails note -->
      <div class="promises-box">
        <ul class="promises hand">
          <li>
            <span class="check-mini"><Icon name="check" size={14} /></span>
            <strong>Zero SMTP / In-App Notification Bar:</strong> Contributors get vouchers immediately without email or spam filters.
          </li>
          <li>
            <span class="check-mini"><Icon name="check" size={14} /></span>
            <strong>Autonomous Bitrefill Execution:</strong> Purchases live gift cards & eSIMs using Bitrefill API & MCP catalog.
          </li>
          <li>
            <span class="check-mini"><Icon name="check" size={14} /></span>
            <strong>Lane 1 Guardrails:</strong> Per-PR spend cap (${currentPool.maxLimitPerPrUsd}) and repo pool balance protections.
          </li>
          <li>
            <span class="check-mini"><Icon name="check" size={14} /></span>
            <strong>Lane 5 Shielded Settlement:</strong> Funded by sponsor ZEC pools with zero public correlation to repo maintainers.
          </li>
        </ul>
      </div>
    </section>

    <!-- Right Column: Interactive GitHub PR Mockup & Bitrefill Settlement Live Stream -->
    <section class="gh-pr-simulator">
      <div class="pr-window">
        <!-- PR Title bar -->
        <div class="pr-titlebar">
          <div class="pr-meta-top">
            <span class="pr-badge-open">
              <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
                <path
                  d="M1.5 3.25a2.25 2.25 0 1 1 3 2.122v5.256a2.251 2.251 0 1 1-1.5 0V5.372A2.25 2.25 0 0 1 1.5 3.25Zm5.677-.177L9.5 5.396l2.323-2.323a.75.75 0 0 1 1.06 1.06L10.56 6.457a.75.75 0 0 1 1.06 0L7.177 4.134a.75.75 0 0 1 1-1.06Z"
                />
              </svg>
              Merged
            </span>
            <span class="pr-repo-label">{repo} #{prNumber}</span>
          </div>
          <h3 class="pr-title">{prTitle}</h3>
        </div>

        <!-- PR Comments Stream -->
        <div class="pr-thread">
          {#each prComments as comment}
            <div class="pr-comment" class:is-bot={comment.author.includes('bot')} use:enter>
              <div class="comment-avatar">
                {#if comment.author.includes('bot')}
                  <div class="bot-avatar-icon">
                    <Icon name="github" size={24} />
                  </div>
                {:else}
                  <img src={comment.avatar} alt={comment.author} />
                {/if}
              </div>
              <div class="comment-bubble">
                <div class="comment-head">
                  <div class="comment-user">
                    <strong>{comment.author}</strong>
                    <span class="comment-role-tag" class:bot-tag={comment.author.includes('bot')}>{comment.badge}</span>
                  </div>
                  <span class="comment-time">{comment.time}</span>
                </div>
                <div class="comment-body">
                  {#if comment.author.includes('bot')}
                    <div class="bot-voucher-card">
                      <div class="bot-card-top">
                        <Art kind="coin-check" />
                        <div>
                          <strong class="voucher-highlight">${voucherAmountUsd} {voucherBrand}</strong>
                          <p class="hand">Purchased via Bitrefill agent rails from repo pool</p>
                        </div>
                      </div>
                      <div class="voucher-details">
                        <div class="detail-row">
                          <span>Recipient Contributor:</span>
                          <strong>@{contributorGithub.replace(/^@/, '')}</strong>
                        </div>
                        <div class="detail-row">
                          <span>Delivery Method:</span>
                          <span class="channel-pill"><Icon name="bell" size={13} /> Notification Bar</span>
                        </div>
                        <div class="detail-row">
                          <span>Bitrefill Invoice ID:</span>
                          <code style="font-size: 0.75rem; color: var(--muted);">{latestInvoiceId}</code>
                        </div>
                        <div class="detail-row">
                          <span>Voucher Code:</span>
                          <span class="code-pill">{bitrefillVoucherCode}</span>
                        </div>
                        <div class="detail-row">
                          <span>Pool Remaining:</span>
                          <strong style="color: var(--terracotta);">${currentPool.poolBalanceUsd.toFixed(2)} USD</strong>
                        </div>
                      </div>
                    </div>
                  {:else if comment.body.startsWith('@zool')}
                    <div class="command-comment">
                      <code>{comment.body}</code>
                    </div>
                  {:else}
                    <p>{comment.body}</p>
                  {/if}
                </div>
              </div>
            </div>
          {/each}

          {#if step === 'processing' || step === 'purchased'}
            <div class="bot-loading-box" use:enter>
              <span class="shimmer-dot"></span>
              <span class="hand">
                {step === 'processing'
                  ? '@zool is calling Bitrefill API and validating guardrails…'
                  : 'Settling invoice via shielded ZEC pool & delivering to notification bar…'}
              </span>
            </div>
          {/if}
        </div>
      </div>
    </section>
  </div>
</main>
