import { test } from 'node:test';
import assert from 'node:assert/strict';
import { organizerAuthorized } from '../src/lib/server/organizer-auth.ts';

test('organizer access requires an exact nonempty server credential', () => {
  assert.equal(organizerAuthorized(null, 'secret'), false);
  assert.equal(organizerAuthorized('Bearer ', ''), false);
  assert.equal(organizerAuthorized('Bearer wrong', 'secret'), false);
  assert.equal(organizerAuthorized('secret', 'secret'), false);
  assert.equal(organizerAuthorized('Bearer secret', 'secret'), true);
});
