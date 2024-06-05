use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
struct SIPrefix {
    name: &'static str,
    symbol: &'static str,
    exp: i8,
    #[allow(dead_code)]
    adoption: u16,
}

/// | Prefix |        | Base 10 | Decimal                          | Adoption |
/// |--------|--------|---------|----------------------------------|----------|
/// | Name   | Symbol |         |                                  |          |
/// |--------|--------|---------|----------------------------------|----------|
/// | quetta | Q      | 10^+30  | 1000000000000000000000000000000  | 2022     |
/// | ronna  | R      | 10^+27  | 1000000000000000000000000000     | 2022     |
/// | yotta  | Y      | 10^+24  | 1000000000000000000000000        | 1991     |
/// | zetta  | Z      | 10^+21  | 1000000000000000000000           | 1991     |
/// | exa    | E      | 10^+18  | 1000000000000000000              | 1975     |
/// | peta   | P      | 10^+15  | 1000000000000000                 | 1975     |
/// | tera   | T      | 10^+12  | 1000000000000                    | 1960     |
/// | giga   | G      | 10^+9   | 1000000000                       | 1960     |
/// | mega   | M      | 10^+6   | 1000000                          | 1873     |
/// | kilo   | k      | 10^+3   | 1000                             | 1795     |
/// | hecto  | h      | 10^2    | 100                              | 1795     |
/// | deca   | da     | 10^1    | 10                               | 1795     |
/// | —      | —      | 10^0    | 1                                | -        |
/// | deci   | d      | 10^−1   | 0.1                              | 1795     |
/// | centi  | c      | 10^−2   | 0.01                             | 1795     |
/// | milli  | m      | 10^−3   | 0.001                            | 1795     |
/// | micro  | μ      | 10^−6   | 0.000001                         | 1873     |
/// | nano   | n      | 10^−9   | 0.000000001                      | 1960     |
/// | pico   | p      | 10^−12  | 0.000000000001                   | 1960     |
/// | femto  | f      | 10^−15  | 0.000000000000001                | 1964     |
/// | atto   | a      | 10^−18  | 0.000000000000000001             | 1964     |
/// | zepto  | z      | 10^−21  | 0.000000000000000000001          | 1991     |
/// | yocto  | y      | 10^−24  | 0.000000000000000000000001       | 1991     |
/// | ronto  | r      | 10^−27  | 0.000000000000000000000000001    | 2022     |
/// | quecto | q      | 10^−30  | 0.000000000000000000000000000001 | 2022     |
const NUMBER_OF_PREFIXES: usize = 24;
#[rustfmt::skip]
const SI_PREFIXES: [SIPrefix; NUMBER_OF_PREFIXES] = [
    SIPrefix { name: "quetta", symbol: "Q",  exp: 30,   adoption: 2022 },
    SIPrefix { name: "ronna",  symbol: "R",  exp: 27,   adoption: 2022 },
    SIPrefix { name: "yotta",  symbol: "Y",  exp: 24,   adoption: 1991 },
    SIPrefix { name: "zetta",  symbol: "Z",  exp: 21,   adoption: 1991 },
    SIPrefix { name: "exa",    symbol: "E",  exp: 18,   adoption: 1975 },
    SIPrefix { name: "peta",   symbol: "P",  exp: 15,   adoption: 1975 },
    SIPrefix { name: "tera",   symbol: "T",  exp: 12,   adoption: 1960 },
    SIPrefix { name: "giga",   symbol: "G",  exp: 9,    adoption: 1960 },
    SIPrefix { name: "mega",   symbol: "M",  exp: 6,    adoption: 1873 },
    SIPrefix { name: "kilo",   symbol: "k",  exp: 3,    adoption: 1795 },
    SIPrefix { name: "hecto",  symbol: "h",  exp: 2,    adoption: 1795 },
    SIPrefix { name: "deca",   symbol: "da", exp: 1,    adoption: 1795 },
//  SIPrefix { name: -,        symbol: -,    exp: 0,    adoption: - },
    SIPrefix { name: "deci",   symbol: "d",  exp: -1,   adoption: 1795 },
    SIPrefix { name: "centi",  symbol: "c",  exp: -2,   adoption: 1795 },
    SIPrefix { name: "milli",  symbol: "m",  exp: -3,   adoption: 1795 },
    SIPrefix { name: "micro",  symbol: "μ",  exp: -6,   adoption: 1873 },
    SIPrefix { name: "nano",   symbol: "n",  exp: -9,   adoption: 1960 },
    SIPrefix { name: "pico",   symbol: "p",  exp: -12,  adoption: 1960 },
    SIPrefix { name: "femto",  symbol: "f",  exp: -15,  adoption: 1964 },
    SIPrefix { name: "atto",   symbol: "a",  exp: -18,  adoption: 1964 },
    SIPrefix { name: "zepto",  symbol: "z",  exp: -21,  adoption: 1991 },
    SIPrefix { name: "yocto",  symbol: "y",  exp: -24,  adoption: 1991 },
    SIPrefix { name: "ronto",  symbol: "r",  exp: -27,  adoption: 2022 },
    SIPrefix { name: "quecto", symbol: "q",  exp: -30,  adoption: 2022 },
];

const SUPERSCRIPTS: [(char, char); 12] = [
    ('0', '⁰'),
    ('1', '¹'),
    ('2', '²'),
    ('3', '³'),
    ('4', '⁴'),
    ('5', '⁵'),
    ('6', '⁶'),
    ('7', '⁷'),
    ('8', '⁸'),
    ('9', '⁹'),
    ('+', '⁺'),
    ('-', '⁻'),
];

pub fn prefix_datapoints() -> [super::Datapoint; NUMBER_OF_PREFIXES] {
    SI_PREFIXES.map(
        |SIPrefix {
             name, symbol, exp, ..
         }| {
            let size = 10_f64.powi(exp.into());
            super::Datapoint {
                name: format!("{name} ({symbol})"),
                size,
                standard_uncertainty: None,
                comment: None,
                refs: vec![],
            }
        },
    )
}

pub fn round_with_scaled_unit(number: f64, unit: &str) -> String {
    let (symbol, scaled_number): (&str, f64) = match pick_prefix(number) {
        Some(prefix) => (prefix.symbol, number / 10_f64.powi(prefix.exp.into())),
        None => ("", number),
    };
    let rounded_number = round_to_three_significant_digits(scaled_number);

    format!("{rounded_number}{symbol}{unit}")
}

pub fn round_with_power(number: f64, unit: &str) -> String {
    const MUL: char = '·';
    // const MUL: char = '×';

    let exp = pick_prefix(number).map(|p| p.exp).unwrap_or(0);

    if exp == 0 {
        let rounded_number = round_to_three_significant_digits(number);
        format!("{rounded_number}{unit}")
    } else {
        let pretty_exp = superscrip(&exp.to_string());

        let scaled_number = number / 10_f64.powi(exp.into());
        let rounded_number = round_to_three_significant_digits(scaled_number);
        if rounded_number == "1" {
            format!("10{pretty_exp}{unit}")
        } else {
            format!("{rounded_number}{MUL}10{pretty_exp}{unit}")
        }
    }
}

fn pick_prefix(number: f64) -> Option<&'static SIPrefix> {
    // 0 would have -inf prefix
    if number == 0. {
        return None;
    }

    let order_of_magnitude = dbg!(number.abs().log10());

    // There is no 'zeroth' prefix
    if (0_f64..3_f64).contains(&order_of_magnitude) {
        return None;
    }

    Some(
        SI_PREFIXES
            .iter()
            .filter(|prefix| prefix.exp % 3 == 0) // Pick one thousand increments.
            .filter(|prefix| f64::from(prefix.exp) <= order_of_magnitude)
            .max_by_key(|prefix| prefix.exp)
            .unwrap_or(&SI_PREFIXES[NUMBER_OF_PREFIXES - 1]),
    )
}

fn round_to_three_significant_digits(number: f64) -> String {
    format!("{number:.3}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}
fn superscrip(value: &str) -> String {
    let map: HashMap<_, _> = SUPERSCRIPTS.into_iter().collect();
    value.chars().map(|c| map.get(&c).unwrap()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pick_prefix() {
        const TESTS: &[(f64, Option<i8>)] = &[
            (0., None),
            (1e0, None),
            (1.1e0, None),
            (1e2, None),
            (9.9e2, None),
            (1e3, Some(3)),
            (11e2, Some(3)),
            (1e4, Some(3)),
            (1e7, Some(6)),
            (1e100, Some(30)),
            //
            (1e-1, Some(-3)),
            (1e-2, Some(-3)),
            (9.9e-2, Some(-3)),
            (1e-3, Some(-3)),
            (11e-2, Some(-3)),
            (0.099e-2, Some(-6)),
            (1e-4, Some(-6)),
            (1e-7, Some(-9)),
            (1e-100, Some(-30)),
        ];
        for &(number, exp) in TESTS {
            println!("{number} / {exp:?}");
            assert_eq!(pick_prefix(number).as_ref().map(|p| p.exp), exp);
            println!("-{number} / {exp:?}");
            assert_eq!(pick_prefix(-number).as_ref().map(|p| p.exp), exp,);
        }
    }
}
