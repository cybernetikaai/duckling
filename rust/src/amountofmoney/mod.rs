//! AmountOfMoney dimension (port of Duckling/AmountOfMoney/Types.hs). The value
//! types are `Currency` and `AmountOfMoneyData`; the English currency/symbol
//! rules, cents composition, intervals, and the shared numeral lift live in
//! `en`. The `currency` field is never optional — it defaults to `Unnamed`
//! (e.g. "bucks"). To add a language, add `amountofmoney/<lang>.rs`.

pub mod en;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Currency {
    // ambiguous
    Cent,
    Dinar,
    Dirham,
    Dollar,
    Pound,
    Rial,
    Riyal,
    Unnamed, // e.g. bucks
    // unambiguous ISO codes
    Aed,
    Aud,
    Bgn,
    Brl,
    Byn,
    Cad,
    Chf,
    Cny,
    Czk,
    Dkk,
    Egp,
    Eur,
    Gbp,
    Hkd,
    Hrk,
    Idr,
    Ils,
    Inr,
    Iqd,
    Jmd,
    Jod,
    Jpy,
    Gel,
    Krw,
    Kwd,
    Lbp,
    Mad,
    Mnt,
    Myr,
    Nok,
    Nzd,
    Pkr,
    Pln,
    Pts,
    Qar,
    Ron,
    Rub,
    Sar,
    Sek,
    Sgd,
    Thb,
    Ttd,
    Usd,
    Vnd,
    Zar,
    Uah,
    Try,
}

impl Currency {
    /// JSON rendering — matches Duckling's `ToJSON Currency`: symbols for the
    /// ambiguous set ("$"/"£"/"cent"/…), "unknown" for Unnamed, and the ISO
    /// code (uppercase) for the rest.
    pub fn as_str(self) -> &'static str {
        use Currency::*;
        match self {
            Cent => "cent",
            Dinar => "dinar",
            Dirham => "dirham",
            Dollar => "$",
            Pound => "\u{00a3}",
            Rial => "rial",
            Riyal => "riyal",
            Unnamed => "unknown",
            Aed => "AED",
            Aud => "AUD",
            Bgn => "BGN",
            Brl => "BRL",
            Byn => "BYN",
            Cad => "CAD",
            Chf => "CHF",
            Cny => "CNY",
            Czk => "CZK",
            Dkk => "DKK",
            Egp => "EGP",
            Eur => "EUR",
            Gbp => "GBP",
            Hkd => "HKD",
            Hrk => "HRK",
            Idr => "IDR",
            Ils => "ILS",
            Inr => "INR",
            Iqd => "IQD",
            Jmd => "JMD",
            Jod => "JOD",
            Jpy => "JPY",
            Gel => "GEL",
            Krw => "KRW",
            Kwd => "KWD",
            Lbp => "LBP",
            Mad => "MAD",
            Mnt => "MNT",
            Myr => "MYR",
            Nok => "NOK",
            Nzd => "NZD",
            Pkr => "PKR",
            Pln => "PLN",
            Pts => "PTS",
            Qar => "QAR",
            Ron => "RON",
            Rub => "RUB",
            Sar => "SAR",
            Sek => "SEK",
            Sgd => "SGD",
            Thb => "THB",
            Ttd => "TTD",
            Usd => "USD",
            Vnd => "VND",
            Zar => "ZAR",
            Uah => "UAH",
            Try => "TRY",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AmountOfMoneyData {
    pub value: Option<f64>,
    pub currency: Currency,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub latent: bool,
}
