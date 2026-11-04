import { defineEnvVars } from '@sveltejs/kit/env';

export const variables = defineEnvVars({
  RESEND_API_KEY: { schema: (value) => value?.trim() || '' },
  ORGANIZER_API_TOKEN: { schema: (value) => value?.trim() || '' },
  RESEND_FROM_EMAIL: {
    schema: (value) => value?.trim() || 'Zool <onboarding@resend.dev>',
  },
});
