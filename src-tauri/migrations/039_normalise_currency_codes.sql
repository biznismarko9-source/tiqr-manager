-- 039: currency columns hold ISO CODES, not symbols.
--
-- WHY THIS EXISTS
--
-- marko's Dashboard was showing "You have data in more than one currency"
-- and offering a button reading "Convert to EUR: € (2)". Converting euros
-- into euros is not a thing, and the banner was not the real damage: every
-- total on that screen is computed for ONE currency, so those two orders,
-- and their tickets and their sales, were being left out of his numbers
-- entirely. Money he owns was invisible.
--
-- The cause is literal. The column held the SYMBOL '€'. Every test in the
-- app asks "is this 'EUR'?", and '€' is not 'EUR'.
--
-- fx.rs's `normalize_currency` already knew about this - its own doc comment
-- describes the same bug being found once before, in the CONVERSION path,
-- and fixed there. What was never done is the obvious other half: the rows
-- already written were left dirty, and nothing stopped a new one being
-- written the same way. 2.64.0 does both. This file is the stored data; the
-- input guard is in commands/orders.rs.
--
-- WHY EVERY TABLE AT ONCE
--
-- Fourteen tables carry a currency. An order normalised to 'EUR' whose
-- tickets still say '€' is a worse state than the one we started in: the
-- two now disagree where before they at least matched. So this is all of
-- them or none.
--
-- WHY THIS IS NOT INVENTING DATA
--
-- Only unambiguous symbols are mapped, and the map is copied from
-- `normalize_currency` so the two can never drift. 'kr' is deliberately
-- absent: it is Swedish, Norwegian AND Danish, and picking one would be
-- guessing about somebody's money. Anything unrecognised is trimmed and
-- upper-cased and otherwise left exactly as it was.
--
-- Every UPDATE is guarded so it only touches rows that actually change. On
-- a database that was already clean this migration writes nothing.

UPDATE accounts
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
UPDATE finance_entries
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
UPDATE market_alerts
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
UPDATE market_snapshots
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
UPDATE orders
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
UPDATE payments
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
UPDATE price_checks
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
UPDATE pulls
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
UPDATE pulls_received
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
UPDATE recurring_expenses
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
UPDATE sales
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
UPDATE ticket_listings
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
UPDATE tickets
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
UPDATE transfers
   SET currency = CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END
 WHERE currency IS NOT NULL
   AND currency <> CASE TRIM(currency)
      WHEN '€'  THEN 'EUR'
      WHEN '$'  THEN 'USD'
      WHEN '£'  THEN 'GBP'
      WHEN '₺'  THEN 'TRY'
      WHEN 'zł' THEN 'PLN'  WHEN 'Zł' THEN 'PLN'  WHEN 'ZŁ' THEN 'PLN'
      WHEN 'лв' THEN 'BGN'
      WHEN 'kč' THEN 'CZK'  WHEN 'Kč' THEN 'CZK'  WHEN 'KČ' THEN 'CZK'
      WHEN 'kc' THEN 'CZK'  WHEN 'Kc' THEN 'CZK'  WHEN 'KC' THEN 'CZK'
      WHEN 'ft' THEN 'HUF'  WHEN 'Ft' THEN 'HUF'  WHEN 'FT' THEN 'HUF'
      WHEN 'lei' THEN 'RON' WHEN 'Lei' THEN 'RON' WHEN 'LEI' THEN 'RON'
      ELSE UPPER(TRIM(currency))
    END;
