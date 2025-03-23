use iso_currency::Currency;
use num_format::{Locale, ToFormattedString as _};

/// Standard number decimal places for the given currency
/// (ex. JPY = 0, USD = 2).
fn decimal_places(currency: Currency) -> usize {
    currency.exponent().unwrap_or(0) as usize
}

/// Format cash amount with currency symbol, correct number of decimal places,
/// proper thousands separators, surrounding parentheses for negative amounts,
/// and optional trailing decimal mark.
///
/// For consistency, uses en locale ('.' as decimal mark, i.e. 1,000.00)
/// regardless of user's locale or currency. Could be generalized in the future.
///
/// For currencies with 0 decimal places, a decimal mark is always included if
/// 'trailing_decimal' is true. For other currencies, this flag has no effect.
pub(crate) fn format_amount(amount: f64, currency: Currency, trailing_decimal: bool) -> String {
    let decimal_places = decimal_places(currency);
    if decimal_places == 0 {
        let amount_rounded = {
            let abs = (amount.abs().round() as i64).to_formatted_string(&Locale::en);
            if amount < 0.0 {
                format!("({})", abs)
            } else {
                abs
            }
        };
        return format!(
            "{}{} {}",
            amount_rounded,
            if trailing_decimal { "." } else { "" },
            currency.symbol()
        );
    } else {
        let amount_integer_part = (amount.abs().trunc() as i64).to_formatted_string(&Locale::en);
        let amount_fractional_part = format!("{:.decimal_places$}", amount.fract())
            .split('.')
            .nth(1)
            .map(|f| f.to_string())
            .unwrap_or_default();
        if amount < 0.0 {
            format!(
                "({}.{:0decimal_places$}) {}",
                amount_integer_part,
                amount_fractional_part,
                currency.symbol(),
            )
        } else {
            format!(
                "{}.{:0decimal_places$} {}",
                amount_integer_part,
                amount_fractional_part,
                currency.symbol(),
            )
        }
    }
}
