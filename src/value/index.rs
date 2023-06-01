use crate::value::Value;
use serde::{de, ser, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Index(Vec<usize>);

impl Index {
    #[inline]
    pub fn as_slice(&self) -> IndexSlice<'_> {
        IndexSlice(&self.0)
    }
}

impl<'de> Deserialize<'de> for Index {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Index, D::Error>
    where
        D: Deserializer<'de>,
    {
        let val: Value = Deserialize::deserialize(deserializer)?;
        match val {
            Value::String(s) => {
                let mut res = Vec::new();
                for v in s.split(',') {
                    let i = v.parse::<usize>().map_err(de::Error::custom)?;
                    res.push(i);
                }
                Ok(Index(res))
            }
            Value::Seq(s) => {
                let mut res = Vec::with_capacity(s.len());
                for v in s {
                    let i = u64::try_from(v).map_err(de::Error::custom)?;
                    let u = usize::try_from(i).map_err(de::Error::custom)?;
                    res.push(u);
                }
                Ok(Index(res))
            }
            _ => {
                if let Ok(v) = u64::try_from(val) {
                    Ok(Index(vec![usize::try_from(v).map_err(de::Error::custom)?]))
                } else {
                    Err(de::Error::custom(
                        "unsupported index (should be integer, list or string)",
                    ))
                }
            }
        }
    }
}

impl Serialize for Index {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0.len() == 1 {
            serializer.serialize_u64(u64::try_from(self.0[0]).map_err(ser::Error::custom)?)
        } else {
            let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
            for idx in &self.0 {
                seq.serialize_element(idx)?;
            }
            seq.end()
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[allow(clippy::module_name_repetitions)]
pub struct IndexSlice<'a>(pub(crate) &'a [usize]);
