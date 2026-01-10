use crate::Error;
use crate::tools::default_true;
use crate::value::Value;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

const ERR_INVALID_RANGE_CONDITION: &str = "Invalid range condition";

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Range {
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default = "default_true")]
    pub min_eq: bool,
    #[serde(default = "default_true")]
    pub max_eq: bool,
}

impl Default for Range {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
            min_eq: true,
            max_eq: true,
        }
    }
}

impl Range {
    #[inline]
    pub fn matches_any(&self) -> bool {
        self.min.is_none() && self.max.is_none()
    }
    pub fn matches_value(&self, val: &Value) -> bool {
        if let Ok(v) = TryInto::<f64>::try_into(val) {
            self.matches(v)
        } else {
            false
        }
    }
    pub fn matches(&self, val: f64) -> bool {
        if let Some(min) = self.min
            && ((self.min_eq && val < min) || (!self.min_eq && val <= min))
        {
            return false;
        }
        if let Some(max) = self.max
            && ((self.max_eq && val > max) || (!self.max_eq && val >= max))
        {
            return false;
        }
        true
    }
    #[inline]
    fn min_eq_sym(&self) -> &'static str {
        if self.min_eq { "<=" } else { "<" }
    }
    #[inline]
    fn max_eq_sym(&self) -> &'static str {
        if self.max_eq { "<=" } else { "<" }
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(min) = self.min {
            if let Some(max) = self.max {
                if (min - max).abs() < f64::EPSILON && self.min_eq && self.max_eq {
                    write!(f, "x = {}", min)
                } else {
                    write!(
                        f,
                        "{} {} x {} {}",
                        min,
                        self.min_eq_sym(),
                        self.max_eq_sym(),
                        max
                    )
                }
            } else {
                write!(f, "{} {} x", min, self.min_eq_sym())
            }
        } else if let Some(max) = self.max {
            write!(f, "x {} {}", self.max_eq_sym(), max)
        } else {
            write!(f, "*")
        }
    }
}

impl FromStr for Range {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let condition = s.trim();
        if condition.is_empty() || condition == "*" || condition == "#" {
            Ok(Range::default())
        } else {
            let mut r_inspected_min: Option<f64> = None;
            let mut r_inspected_max: Option<f64> = None;
            let mut r_inspected_min_eq = false;
            let mut r_inspected_max_eq = false;
            let c = condition
                .replace(' ', "")
                .replace(">=", "}")
                .replace("=>", "}")
                .replace("<=", "{")
                .replace("=<", "{")
                .replace("===", "=")
                .replace("==", "=");
            let vals = c
                .split(&['<', '>', '}', '{', '='][..])
                .collect::<Vec<&str>>();
            if vals.len() > 3 {
                return Err(Error::invalid_data(ERR_INVALID_RANGE_CONDITION));
            }
            if vals.len() > 1 {
                for (i, v) in vals.iter().enumerate() {
                    if *v == "x" || *v == "X" {
                        if vals.len() == 2 {
                            if i > 1 {
                                return Err(Error::invalid_data(ERR_INVALID_RANGE_CONDITION));
                            }
                            let s = c
                                .chars()
                                .nth(vals[0].len())
                                .ok_or_else(|| Error::invalid_data(ERR_INVALID_RANGE_CONDITION))?;
                            if s == '=' {
                                r_inspected_min = Some(vals[1 - i].parse()?);
                                r_inspected_max = r_inspected_min;
                                r_inspected_min_eq = true;
                                r_inspected_max_eq = true;
                            } else if ((s == '}' || s == '>') && i == 0)
                                || ((s == '{' || s == '<') && i == 1)
                            {
                                r_inspected_min = Some(vals[1 - i].parse()?);
                                r_inspected_min_eq = s == '}' || s == '{';
                            } else if ((s == '}' || s == '>') && i == 1)
                                || ((s == '{' || s == '<') && i == 0)
                            {
                                r_inspected_max = Some(vals[1 - i].parse()?);
                                r_inspected_max_eq = s == '}' || s == '{';
                            }
                        } else if vals.len() == 3 {
                            if i != 1 {
                                return Err(Error::invalid_data(ERR_INVALID_RANGE_CONDITION));
                            }
                            let s1_ch = c.chars().nth(vals[0].len());
                            let s2_ch = c.chars().nth(vals[0].len() + 2);
                            if let Some(s1) = s1_ch
                                && let Some(s2) = s2_ch
                            {
                                if s2 == '}' || s2 == '>' {
                                    r_inspected_max = Some(vals[i - 1].parse()?);
                                    r_inspected_max_eq = s1 == '}';
                                    r_inspected_min = Some(vals[i + 1].parse()?);
                                    r_inspected_min_eq = s2 == '}';
                                } else if s2 == '{' || s2 == '<' {
                                    r_inspected_min = Some(vals[i - 1].parse()?);
                                    r_inspected_min_eq = s1 == '{';
                                    r_inspected_max = Some(vals[i + 1].parse()?);
                                    r_inspected_max_eq = s2 == '{';
                                }
                            }
                            if r_inspected_max.unwrap() <= r_inspected_min.unwrap() {
                                return Err(Error::invalid_data(ERR_INVALID_RANGE_CONDITION));
                            }
                        }
                        break;
                    }
                }
            } else {
                return Err(Error::invalid_data(ERR_INVALID_RANGE_CONDITION));
            }
            Ok(Self {
                min: r_inspected_min,
                max: r_inspected_max,
                min_eq: r_inspected_min_eq,
                max_eq: r_inspected_max_eq,
            })
        }
    }
}

pub fn de_range<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = Error>,
    D: Deserializer<'de>,
{
    struct StringOrStruct<T>(PhantomData<fn() -> T>);

    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = Error>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            Ok(FromStr::from_str(value).unwrap())
        }
        fn visit_string<E>(self, value: String) -> Result<T, E>
        where
            E: de::Error,
        {
            Ok(FromStr::from_str(&value).unwrap())
        }
        fn visit_map<M>(self, map: M) -> Result<T, M::Error>
        where
            M: MapAccess<'de>,
        {
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

pub fn de_opt_range<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = Error>,
    D: Deserializer<'de>,
{
    struct StringOrStruct<T>(PhantomData<fn() -> Option<T>>);

    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = Error>,
    {
        type Value = Option<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_unit<E>(self) -> Result<Option<T>, E>
        where
            E: de::Error,
        {
            Ok(None)
        }
        fn visit_str<E>(self, value: &str) -> Result<Option<T>, E>
        where
            E: de::Error,
        {
            Ok(Some(FromStr::from_str(value).unwrap()))
        }
        fn visit_string<E>(self, value: String) -> Result<Option<T>, E>
        where
            E: de::Error,
        {
            Ok(Some(FromStr::from_str(&value).unwrap()))
        }
        fn visit_map<M>(self, map: M) -> Result<Option<T>, M::Error>
        where
            M: MapAccess<'de>,
        {
            let res = Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))?;
            Ok(Some(res))
        }
    }

    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

#[cfg(test)]
mod test {
    use super::{Range, de_opt_range, de_range};
    use serde::Deserialize;

    #[test]
    fn test_de() {
        #[derive(Deserialize)]
        struct TestR {
            #[serde(deserialize_with = "de_range")]
            range: Range,
        }
        let range = Range {
            min: Some(20.0),
            max: Some(100.0),
            min_eq: true,
            max_eq: false,
        };
        let rs = r#"{ "range": {"min": 20, "max": 100, "min_eq": true, "max_eq": false }}"#;
        let rdes: TestR = serde_json::from_str(rs).unwrap();
        assert_eq!(rdes.range, range);
        let rs = r#"{ "range": "20 <=x < 100" }"#;
        let rdes: TestR = serde_json::from_str(rs).unwrap();
        assert_eq!(rdes.range, range);
    }

    #[test]
    fn test_opt_de() {
        #[derive(Deserialize)]
        struct TestR {
            #[serde(default, deserialize_with = "de_opt_range")]
            range: Option<Range>,
        }
        let range = Range {
            min: Some(20.0),
            max: Some(100.0),
            min_eq: true,
            max_eq: false,
        };
        let rs = r#"{ "range": {"min": 20, "max": 100, "min_eq": true, "max_eq": false }}"#;
        let rdes: TestR = serde_json::from_str(rs).unwrap();
        assert_eq!(rdes.range.unwrap(), range);
        let rs = r#"{ "range": "20 <=x < 100" }"#;
        let rdes: TestR = serde_json::from_str(rs).unwrap();
        assert_eq!(rdes.range.unwrap(), range);
        let rs = r#"{ "range": null }"#;
        let rdes: TestR = serde_json::from_str(rs).unwrap();
        assert!(rdes.range.is_none());
        let rs = r"{}";
        let rdes: TestR = serde_json::from_str(rs).unwrap();
        assert!(rdes.range.is_none());
    }

    #[test]
    fn test_range() {
        let r = Range::default();
        let r2 = Range::default();
        assert_eq!(r, r2);
        assert!(r.matches(111.0));
        assert_eq!(r.to_string(), "*");
        assert_eq!(r, "*".parse().unwrap());
        let mut r = Range {
            min: Some(0.0),
            max: None,
            min_eq: true,
            max_eq: false,
        };
        let r2 = r;
        assert_eq!(r, r2);
        assert!(r.matches(1.0));
        assert!(r.matches(0.0));
        assert!(!r.matches(-1.0));
        assert_eq!(r.to_string(), "0 <= x");
        assert_eq!(r, "0 <= x".parse().unwrap());
        assert_eq!(r, "x >= 0".parse().unwrap());
        assert_eq!(r, "x => 0".parse().unwrap());
        r.min_eq = false;
        assert!(r.matches(1.0));
        assert!(!r.matches(0.0));
        assert!(!r.matches(-1.0));
        assert_eq!(r.to_string(), "0 < x");
        assert_eq!(r, "0 < x".parse().unwrap());
        assert_eq!(r, "x>0".parse().unwrap());
        r.max = Some(100.0);
        assert!(r.matches(1.0));
        assert!(!r.matches(0.0));
        assert!(!r.matches(-1.0));
        assert!(r.matches(99.0));
        assert!(!r.matches(100.0));
        assert!(!r.matches(101.0));
        assert_eq!(r.to_string(), "0 < x < 100");
        assert_eq!(r, "0 < x < 100".parse().unwrap());
        assert_eq!(r, "100>x > 0".parse().unwrap());
        r.max_eq = true;
        assert!(r.matches(1.0));
        assert!(!r.matches(0.0));
        assert!(!r.matches(-1.0));
        assert!(r.matches(99.0));
        assert!(r.matches(100.0));
        assert!(!r.matches(101.0));
        assert_eq!(r.to_string(), "0 < x <= 100");
        assert_eq!(r, "0 < x <= 100".parse().unwrap());
        assert_eq!(r, "100=>x > 0".parse().unwrap());
        r.min_eq = true;
        assert_eq!(r.to_string(), "0 <= x <= 100");
        assert_eq!(r, "0 <= x <= 100".parse().unwrap());
        assert_eq!(r, "100=>x=> 0".parse().unwrap());
        let r = Range {
            min: None,
            max: Some(100.0),
            min_eq: false,
            max_eq: true,
        };
        assert!(r.matches(1.0));
        assert!(r.matches(0.0));
        assert!(r.matches(-1.0));
        assert!(r.matches(99.0));
        assert!(r.matches(100.0));
        assert!(!r.matches(101.0));
        assert_eq!(r.to_string(), "x <= 100");
        assert_eq!(r, "x <= 100".parse().unwrap());
        assert_eq!(r, "100>=x".parse().unwrap());
    }
}
