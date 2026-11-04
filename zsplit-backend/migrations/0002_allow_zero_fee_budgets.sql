-- Preserve the checksum of the already-applied initial migration.
-- Zero fee budgets are an existing application option; change constraints forward.
ALTER TABLE rooms DROP CONSTRAINT rooms_fee_reserve_zat_check;
ALTER TABLE rooms ADD CONSTRAINT rooms_fee_reserve_zat_check CHECK (fee_reserve_zat >= 0);
ALTER TABLE claims DROP CONSTRAINT claims_fee_budget_zat_check;
ALTER TABLE claims ADD CONSTRAINT claims_fee_budget_zat_check CHECK (fee_budget_zat >= 0);
