CREATE TABLE organizers (
 id UUID PRIMARY KEY, auth_subject TEXT NOT NULL UNIQUE, created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE rooms (
 id UUID PRIMARY KEY, organizer_id UUID NOT NULL REFERENCES organizers(id),
 network TEXT NOT NULL CHECK(network IN ('mainnet','testnet','regtest')),
 state TEXT NOT NULL DEFAULT 'awaiting_funds' CHECK(state IN ('awaiting_funds','ready','paused','closed')),
 deposit_address TEXT NOT NULL, birthday_height BIGINT NOT NULL CHECK(birthday_height >= 0),
 payout_amounts JSONB NOT NULL, fee_reserve_zat BIGINT NOT NULL CHECK(fee_reserve_zat > 0),
 confirmed_zat BIGINT NOT NULL DEFAULT 0 CHECK(confirmed_zat >= 0),
 spendable_zat BIGINT NOT NULL DEFAULT 0 CHECK(spendable_zat >= 0),
 reserved_zat BIGINT NOT NULL DEFAULT 0 CHECK(reserved_zat >= 0),
 chain_height BIGINT NOT NULL DEFAULT 0, scanned_height BIGINT NOT NULL DEFAULT 0,
 balance_updated_at TIMESTAMPTZ, claims_issued BOOLEAN NOT NULL DEFAULT false,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX rooms_owner ON rooms(organizer_id,id);
CREATE TABLE claims (
 id UUID PRIMARY KEY, room_id UUID NOT NULL REFERENCES rooms(id),
 kind TEXT NOT NULL DEFAULT 'claim' CHECK(kind IN ('claim','withdrawal')),
 amount_zat BIGINT NOT NULL CHECK(amount_zat > 0 AND amount_zat <= 2100000000000000),
 fee_budget_zat BIGINT NOT NULL CHECK(fee_budget_zat > 0),
 token_digest BYTEA UNIQUE CHECK(token_digest IS NULL OR octet_length(token_digest)=32),
 state TEXT NOT NULL DEFAULT 'reserved' CHECK(state IN ('reserved','funding_queued','preparing','funding_broadcast','funded_unconfirmed','funded_confirmed','expired','cancelled','ambiguous_broadcast','reconciling','reorged','manual_review')),
 expires_at TIMESTAMPTZ NOT NULL, payout_id UUID UNIQUE,
 issued_at TIMESTAMPTZ NOT NULL DEFAULT now(), version BIGINT NOT NULL DEFAULT 0,
 sweep_reported BOOLEAN NOT NULL DEFAULT false,
 reservation_released BOOLEAN NOT NULL DEFAULT false,
 CHECK((kind='claim' AND token_digest IS NOT NULL) OR (kind='withdrawal' AND token_digest IS NULL))
);
CREATE INDEX claims_room_state ON claims(room_id,state);
CREATE INDEX claims_expiry ON claims(expires_at) WHERE state='reserved';
CREATE TABLE payout_jobs (
 id UUID PRIMARY KEY, claim_id UUID NOT NULL UNIQUE REFERENCES claims(id), room_id UUID NOT NULL REFERENCES rooms(id),
 state TEXT NOT NULL DEFAULT 'queued' CHECK(state IN ('queued','preparing','prepared','funding_broadcast','funded_unconfirmed','funded_confirmed','ambiguous_broadcast','reorged','reconciling','manual_review')),
 idempotency_key UUID NOT NULL, encrypted_ephemeral_receiver BYTEA,
 request_fingerprint BYTEA NOT NULL, txid TEXT UNIQUE, raw_transaction BYTEA,
 attempts INTEGER NOT NULL DEFAULT 0 CHECK(attempts >= 0),
 next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(), lease_until TIMESTAMPTZ, lease_owner UUID,
 error_code TEXT, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 UNIQUE(room_id,idempotency_key), CHECK((txid IS NULL)=(raw_transaction IS NULL))
);
ALTER TABLE claims ADD CONSTRAINT claims_payout_fk FOREIGN KEY(payout_id) REFERENCES payout_jobs(id) DEFERRABLE INITIALLY DEFERRED;
CREATE INDEX payout_jobs_queue ON payout_jobs(next_attempt_at,created_at) WHERE state IN ('queued','prepared');
CREATE TABLE wallet_transactions (
 txid TEXT PRIMARY KEY, payout_id UUID NOT NULL UNIQUE REFERENCES payout_jobs(id), network TEXT NOT NULL,
 block_height BIGINT, confirmations INTEGER NOT NULL DEFAULT 0 CHECK(confirmations>=0), state TEXT NOT NULL, updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE wallet_deposits (
 id UUID PRIMARY KEY, room_id UUID NOT NULL REFERENCES rooms(id), txid TEXT NOT NULL,
 amount_zat BIGINT NOT NULL CHECK(amount_zat>0), block_height BIGINT, confirmations INTEGER NOT NULL CHECK(confirmations>=0), state TEXT NOT NULL,
 UNIQUE(room_id,txid)
);
CREATE TABLE ledger_entries (
 id BIGSERIAL PRIMARY KEY, room_id UUID NOT NULL REFERENCES rooms(id), claim_id UUID REFERENCES claims(id),
 kind TEXT NOT NULL CHECK(kind IN ('reserve','release_expiry','release_confirmed','queue','withdraw_reserve','reserve_reorg')),
 amount_zat BIGINT NOT NULL CHECK(amount_zat>=0), created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE audit_events (
 id BIGSERIAL PRIMARY KEY, room_id UUID NOT NULL REFERENCES rooms(id), claim_id UUID REFERENCES claims(id),
 event_type TEXT NOT NULL, actor_type TEXT NOT NULL CHECK(actor_type IN ('organizer','bearer','worker')),
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE FUNCTION prevent_claim_rewrite() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.amount_zat <> OLD.amount_zat OR NEW.token_digest IS DISTINCT FROM OLD.token_digest OR NEW.room_id <> OLD.room_id OR NEW.fee_budget_zat <> OLD.fee_budget_zat THEN
  RAISE EXCEPTION 'immutable claim fields';
 END IF;
 RETURN NEW;
END; $$;
CREATE TRIGGER immutable_claim_fields BEFORE UPDATE ON claims FOR EACH ROW EXECUTE FUNCTION prevent_claim_rewrite();
CREATE FUNCTION prevent_ledger_mutation() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'immutable ledger'; END; $$;
CREATE TRIGGER immutable_ledger BEFORE UPDATE OR DELETE ON ledger_entries FOR EACH ROW EXECUTE FUNCTION prevent_ledger_mutation();
