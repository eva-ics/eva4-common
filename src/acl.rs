use crate::value::to_value;
use crate::{is_str_any, is_str_wildcard, EResult, Error, ItemKind, Value, OID};
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use submap::AclMap;

static ERR_INVALID_OID_MASK: &str = "Invalid OID mask format";
static ERR_PATH_MASK_EMPTY: &str = "Empty path mask";
static ERR_INVALID_OID_MASK_OP: &str = "Invalid OID mask for this op";

#[inline]
pub fn create_acl_map() -> AclMap {
    AclMap::new()
        .separator('/')
        .wildcard_multiple(crate::WILDCARD)
        .match_any_multiple(crate::MATCH_ANY)
}

#[derive(Debug, Clone, Eq)]
pub struct PathMask {
    chunks: Option<Vec<String>>,
}

impl PathMask {
    #[inline]
    fn new_any() -> Self {
        Self { chunks: None }
    }
    #[inline]
    fn is_any(&self) -> bool {
        self.chunks.is_none()
    }
    fn matches_split(&self, path_split: &mut std::str::Split<'_, char>) -> bool {
        if let Some(ref chunks) = self.chunks {
            let mut s_m = chunks.iter();
            loop {
                if let Some(i_chunk) = path_split.next() {
                    if let Some(m_chunk) = s_m.next() {
                        if is_str_wildcard(m_chunk) {
                            return true;
                        }
                        if !is_str_any(m_chunk) && i_chunk != m_chunk {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else {
                    return s_m.next().is_none();
                }
            }
        } else {
            true
        }
    }
}

impl<'de> Deserialize<'de> for PathMask {
    fn deserialize<D>(deserializer: D) -> Result<PathMask, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_unit(PathMaskVisitor)
    }
}

struct PathMaskVisitor;
impl<'de> serde::de::Visitor<'de> for PathMaskVisitor {
    type Value = PathMask;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string-packed path mask")
    }
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        value
            .parse()
            .map_err(|e| E::custom(format!("{}: {}", e, value)))
    }
    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        value
            .parse()
            .map_err(|e| E::custom(format!("{}: {}", e, value)))
    }
}

#[derive(Debug, Clone, Default)]
pub struct PathMaskList {
    acl_map: AclMap,
}

impl Serialize for PathMaskList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let path_masks = self.acl_map.list();
        let mut seq = serializer.serialize_seq(Some(path_masks.len()))?;
        for el in path_masks {
            seq.serialize_element(el)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for PathMaskList {
    fn deserialize<D>(deserializer: D) -> Result<PathMaskList, D::Error>
    where
        D: Deserializer<'de>,
    {
        let masks: Vec<String> = Deserialize::deserialize(deserializer)?;
        Ok(PathMaskList::from_string_list(&masks))
    }
}

impl From<PathMaskList> for Value {
    fn from(v: PathMaskList) -> Value {
        to_value(v).unwrap()
    }
}

impl TryFrom<Value> for PathMaskList {
    type Error = Error;
    fn try_from(value: Value) -> EResult<PathMaskList> {
        match value {
            Value::Seq(_) => {
                let masks: Vec<String> = value.deserialize_into()?;
                Ok(PathMaskList::from_string_list(&masks))
            }
            Value::String(s) => {
                if s.is_empty() {
                    Ok(<_>::default())
                } else {
                    Ok(PathMaskList::from_str_list(
                        &s.split(',').collect::<Vec<_>>(),
                    ))
                }
            }
            _ => Err(Error::invalid_data("Expected vec or string")),
        }
    }
}

impl PathMaskList {
    pub fn from_str_list(s_masks: &[&str]) -> Self {
        let mut acl_map = create_acl_map();
        for s in s_masks {
            if !s.is_empty() {
                acl_map.insert(s);
            }
        }
        Self { acl_map }
    }
    pub fn from_string_list(s_masks: &[String]) -> Self {
        let mut acl_map = create_acl_map();
        for s in s_masks {
            if !s.is_empty() {
                acl_map.insert(s);
            }
        }
        Self { acl_map }
    }
    #[inline]
    pub fn matches(&self, path: &str) -> bool {
        self.acl_map.matches(path)
    }
    pub fn is_empty(&self) -> bool {
        self.acl_map.is_empty()
    }
}

impl PartialEq for PathMask {
    fn eq(&self, other: &Self) -> bool {
        self.chunks == other.chunks
    }
}

impl Ord for PathMask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.chunks.cmp(&other.chunks)
    }
}

impl Hash for PathMask {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.chunks.hash(hasher);
    }
}

impl PartialOrd for PathMask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for PathMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref chunks) = self.chunks {
            write!(f, "{}", chunks.join("/"))
        } else {
            write!(f, "#")
        }
    }
}

impl FromStr for PathMask {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            Err(Error::invalid_data(ERR_PATH_MASK_EMPTY))
        } else if is_str_wildcard(s) {
            Ok(Self::new_any())
        } else {
            let mut chunks = Vec::new();
            for chunk in s.split('/') {
                if is_str_wildcard(chunk) {
                    chunks.push("#".to_owned());
                    break;
                }
                chunks.push(chunk.to_owned());
            }
            Ok(Self {
                chunks: Some(chunks),
            })
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OIDMaskList {
    oid_masks: HashSet<OIDMask>,
    acl_map: AclMap,
}

impl Serialize for OIDMaskList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.oid_masks.len()))?;
        for element in &self.oid_masks {
            seq.serialize_element(&element.to_string())?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for OIDMaskList {
    fn deserialize<D>(deserializer: D) -> Result<OIDMaskList, D::Error>
    where
        D: Deserializer<'de>,
    {
        let masks: HashSet<OIDMask> = Deserialize::deserialize(deserializer)?;
        Ok(OIDMaskList::new(masks))
    }
}

impl FromIterator<OIDMask> for OIDMaskList {
    fn from_iter<I>(masks: I) -> Self
    where
        I: IntoIterator<Item = OIDMask>,
    {
        let mut s: HashSet<OIDMask> = HashSet::new();
        for mask in masks {
            s.insert(mask);
        }
        Self::new(s)
    }
}

impl OIDMaskList {
    #[inline]
    pub fn new(oid_masks: HashSet<OIDMask>) -> Self {
        let mut acl_map = create_acl_map();
        for mask in &oid_masks {
            acl_map.insert(&mask.as_path());
        }
        Self { oid_masks, acl_map }
    }
    #[inline]
    pub fn new0(oid_mask: OIDMask) -> Self {
        let mut acl_map = create_acl_map();
        acl_map.insert(&oid_mask.as_path());
        let mut oid_masks = HashSet::new();
        oid_masks.insert(oid_mask);
        Self { oid_masks, acl_map }
    }
    #[inline]
    pub fn new_any() -> Self {
        let mut acl_map = create_acl_map();
        acl_map.insert(crate::WILDCARD[0]);
        let mut oid_masks = HashSet::new();
        oid_masks.insert(OIDMask::new_any());
        Self { oid_masks, acl_map }
    }
    pub fn from_str_list(s_masks: &[&str]) -> EResult<Self> {
        let mut oid_masks = HashSet::new();
        for s in s_masks {
            oid_masks.insert(s.parse()?);
        }
        Ok(Self::new(oid_masks))
    }
    pub fn from_string_list(s_masks: &[String]) -> EResult<Self> {
        let mut oid_masks = HashSet::new();
        for s in s_masks {
            oid_masks.insert(s.parse()?);
        }
        Ok(Self::new(oid_masks))
    }
    #[inline]
    pub fn matches(&self, oid: &OID) -> bool {
        self.acl_map.matches(oid.as_path())
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.oid_masks.is_empty()
    }
    #[inline]
    pub fn oid_masks(&self) -> &HashSet<OIDMask> {
        &self.oid_masks
    }
    #[inline]
    pub fn as_string_vec(&self) -> Vec<String> {
        self.oid_masks.iter().map(ToString::to_string).collect()
    }
    pub fn try_from_iter<I, T>(values: I) -> EResult<Self>
    where
        I: IntoIterator<Item = T>,
        T: TryInto<OIDMask, Error = Error>,
    {
        let mut res = HashSet::new();
        for v in values {
            let mask: OIDMask = v.try_into()?;
            res.insert(mask);
        }
        Ok(Self::new(res))
    }
}

#[derive(Debug, Clone, Eq)]
pub struct OIDMask {
    kind: Option<ItemKind>,
    path: PathMask,
}

impl OIDMask {
    #[inline]
    fn check(s: &str) -> EResult<()> {
        if s.len() > 65000 {
            return Err(Error::invalid_data("OID mask too long"));
        }
        for c in s.chars() {
            if !(c.is_alphanumeric() || crate::OID_MASK_ALLOWED_SYMBOLS.contains(c) || c == '/') {
                return Err(Error::invalid_data(format!(
                    "Invalid symbol in OID mask: {}",
                    c
                )));
            }
        }
        Ok(())
    }
    #[inline]
    pub fn kind(&self) -> Option<ItemKind> {
        self.kind
    }
    /// A special case, when OID mask can be converted to "wildcard OID" - an OID, where id is the
    /// wildcard symbol. Wildcard OIDs are special types of OIDs, which are fully compatible with
    /// majority of ACL checkers and can be used to obtain data from various database sources,
    /// which support wildcard selections (such as like 'kind:group/%' in SQL
    #[inline]
    pub fn to_wildcard_oid(&self) -> EResult<OID> {
        if let Some(kind) = self.kind {
            if let Some(ref ch) = self.path.chunks {
                for (i, p) in ch.iter().enumerate() {
                    if is_str_any(p) || (is_str_wildcard(p) && i < p.len()) {
                        return Err(Error::invalid_data(ERR_INVALID_OID_MASK_OP));
                    }
                }
            }
            Ok(OID::new0_unchecked(kind, &self.path.to_string())?)
        } else {
            Err(Error::invalid_data(ERR_INVALID_OID_MASK_OP))
        }
    }
    fn parse_oid_mask(s: &str, c: char) -> EResult<Self> {
        if is_str_wildcard(s) {
            Ok(Self::new_any())
        } else {
            s.find(c).map_or_else(
                || {
                    let kind: ItemKind = s.parse()?;
                    Ok(OIDMask {
                        kind: Some(kind),
                        path: PathMask::new_any(),
                    })
                },
                |tpos| {
                    if tpos == s.len() {
                        Err(Error::invalid_data(format!(
                            "{}: {}",
                            ERR_INVALID_OID_MASK, s
                        )))
                    } else {
                        let tp_str = &s[..tpos];
                        let kind: Option<ItemKind> = if is_str_any(tp_str) {
                            None
                        } else {
                            Some(s[..tpos].parse()?)
                        };
                        let p = &s[tpos + 1..];
                        OIDMask::check(p)?;
                        Ok(OIDMask {
                            kind,
                            path: p.parse()?,
                        })
                    }
                },
            )
        }
    }
    #[inline]
    pub fn from_path(s: &str) -> EResult<Self> {
        Self::parse_oid_mask(s, '/')
    }
    #[inline]
    pub fn as_path(&self) -> String {
        if self.path.chunks.is_some() {
            format!(
                "{}/{}",
                if let Some(ref kind) = self.kind {
                    kind.as_str()
                } else {
                    "+"
                },
                self.path
            )
        } else if let Some(ref kind) = self.kind {
            format!("{}/#", kind.as_str())
        } else {
            "#".to_owned()
        }
    }
    #[inline]
    pub fn chunks(&self) -> Option<Vec<&str>> {
        self.path
            .chunks
            .as_ref()
            .map(|v| v.iter().map(String::as_str).collect())
    }
    #[inline]
    pub fn new_any() -> Self {
        OIDMask {
            kind: None,
            path: PathMask::new_any(),
        }
    }
    pub fn matches(&self, oid: &OID) -> bool {
        let oid_tp = oid.kind();
        let sp = oid.full_id().split('/');
        if let Some(mask_tp) = self.kind {
            if mask_tp != oid_tp {
                return false;
            }
        }
        if self.path.matches_split(&mut sp.clone()) {
            return true;
        }
        false
    }
}

impl PartialEq for OIDMask {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.path == other.path
    }
}

impl Ord for OIDMask {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.kind == other.kind {
            self.path.cmp(&other.path)
        } else {
            self.kind.cmp(&other.kind)
        }
    }
}

impl fmt::Display for OIDMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(kind) = self.kind {
            write!(f, "{}:{}", kind, self.path)
        } else if self.path.is_any() {
            write!(f, "#")
        } else {
            write!(f, "+:{}", self.path)
        }
    }
}

impl From<Vec<OIDMask>> for OIDMaskList {
    fn from(v: Vec<OIDMask>) -> Self {
        Self::from_iter(v)
    }
}

impl From<OID> for OIDMask {
    fn from(oid: OID) -> Self {
        OIDMask {
            kind: Some(oid.kind()),
            path: oid.full_id().parse().unwrap(),
        }
    }
}

impl From<OID> for OIDMaskList {
    fn from(oid: OID) -> Self {
        let mask = OIDMask {
            kind: Some(oid.kind()),
            path: oid.full_id().parse().unwrap(),
        };
        mask.into()
    }
}

impl FromStr for OIDMask {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_oid_mask(s, ':')
    }
}

macro_rules! impl_oidmask_from_str {
    ($t: ty) => {
        impl TryFrom<$t> for OIDMask {
            type Error = Error;
            fn try_from(s: $t) -> EResult<Self> {
                s.parse()
            }
        }
    };
}

impl_oidmask_from_str!(String);
impl_oidmask_from_str!(&str);
impl_oidmask_from_str!(&&str);

impl Hash for OIDMask {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.kind.map_or(0, |v| v as u16).hash(hasher);
        self.path.hash(hasher);
    }
}

impl PartialOrd for OIDMask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'de> Deserialize<'de> for OIDMask {
    fn deserialize<D>(deserializer: D) -> Result<OIDMask, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for OIDMask {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<OIDMaskList> for Value {
    fn from(v: OIDMaskList) -> Value {
        to_value(v).unwrap()
    }
}

impl From<OIDMask> for OIDMaskList {
    fn from(mask: OIDMask) -> Self {
        OIDMaskList::new0(mask)
    }
}

impl TryFrom<Value> for OIDMaskList {
    type Error = Error;
    fn try_from(value: Value) -> EResult<OIDMaskList> {
        match value {
            Value::Seq(_) => {
                let masks: Vec<String> = value.deserialize_into()?;
                Ok(OIDMaskList::from_string_list(&masks)?)
            }
            Value::String(s) => {
                if s.is_empty() {
                    Ok(<_>::default())
                } else {
                    Ok(OIDMaskList::from_str_list(
                        &s.split(',').collect::<Vec<_>>(),
                    )?)
                }
            }
            _ => Err(Error::invalid_data("Expected vec or string")),
        }
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Copy, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Op {
    Log,
    Moderator,
    Supervisor,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Op::Log => "log",
                Op::Moderator => "moderator",
                Op::Supervisor => "supervisor",
            }
        )
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
struct AclItemsPvt {
    #[serde(default)]
    items: OIDMaskList,
    #[serde(default)]
    pvt: PathMaskList,
    #[serde(default)]
    rpvt: PathMaskList,
}

//#[derive(Serialize, Deserialize, Default, Clone, Debug)]
//struct AclItems {
//#[serde(default)]
//items: OIDMaskList,
//}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(val: &bool) -> bool {
    !val
}

/// The default ACL, used by most of services. Can be overriden with a custom one
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Acl {
    id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    admin: bool,
    #[serde(default)]
    read: AclItemsPvt,
    #[serde(default)]
    write: AclItemsPvt,
    #[serde(default)]
    deny_read: AclItemsPvt,
    #[serde(default, alias = "deny")]
    deny_write: AclItemsPvt,
    #[serde(default)]
    ops: HashSet<Op>,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<Value>,
    from: Vec<String>,
}

impl Acl {
    #[inline]
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn get_items_allow_deny_reading(&self) -> (Vec<String>, Vec<String>) {
        if self.admin {
            (vec!["#".to_owned()], vec![])
        } else {
            let mut allow: HashSet<String> = self.read.items.as_string_vec().into_iter().collect();
            let allow_write: HashSet<String> =
                self.write.items.as_string_vec().into_iter().collect();
            allow.extend(allow_write);
            let deny: HashSet<String> = self.deny_read.items.as_string_vec().into_iter().collect();
            (allow.into_iter().collect(), deny.into_iter().collect())
        }
    }
    #[inline]
    pub fn check_admin(&self) -> bool {
        self.admin
    }
    #[inline]
    pub fn check_op(&self, op: Op) -> bool {
        self.admin || self.ops.contains(&op)
    }
    #[inline]
    pub fn check_item_read(&self, oid: &OID) -> bool {
        self.admin
            || ((self.read.items.matches(oid) || self.write.items.matches(oid))
                && !self.deny_read.items.matches(oid))
    }
    #[inline]
    pub fn check_item_write(&self, oid: &OID) -> bool {
        self.admin
            || (self.write.items.matches(oid)
                && !self.deny_write.items.matches(oid)
                && !self.deny_read.items.matches(oid))
    }
    #[inline]
    pub fn check_pvt_read(&self, path: &str) -> bool {
        self.admin || (self.read.pvt.matches(path) && !self.deny_read.pvt.matches(path))
    }
    #[inline]
    pub fn check_pvt_write(&self, path: &str) -> bool {
        self.admin
            || (self.write.pvt.matches(path)
                && !self.deny_write.pvt.matches(path)
                && !self.deny_read.pvt.matches(path))
    }
    #[inline]
    pub fn check_rpvt_read(&self, path: &str) -> bool {
        if self.admin {
            true
        } else {
            let mut sp = path.splitn(2, '/');
            if let Some(node) = sp.next() {
                if let Some(uri) = sp.next() {
                    let stripped_uri = if let Some(u) = uri.strip_prefix("https://") {
                        u
                    } else if let Some(u) = uri.strip_prefix("http://") {
                        u
                    } else {
                        uri
                    };
                    let stripped_path = format!("{node}/{stripped_uri}");
                    self.read.rpvt.matches(&stripped_path)
                        && !self.deny_read.rpvt.matches(&stripped_path)
                } else {
                    false
                }
            } else {
                false
            }
        }
    }
    #[inline]
    pub fn require_admin(&self) -> EResult<()> {
        if self.check_admin() {
            Ok(())
        } else {
            Err(Error::access("admin access required"))
        }
    }
    pub fn require_op(&self, op: Op) -> EResult<()> {
        if self.check_op(op) {
            Ok(())
        } else {
            Err(Error::access(format!("operation access required: {}", op)))
        }
    }
    pub fn require_item_read(&self, oid: &OID) -> EResult<()> {
        if self.check_item_read(oid) {
            Ok(())
        } else {
            Err(Error::access(format!("read access required for: {}", oid)))
        }
    }
    pub fn require_item_write(&self, oid: &OID) -> EResult<()> {
        if self.check_item_write(oid) {
            Ok(())
        } else {
            Err(Error::access(format!("write access required for: {}", oid)))
        }
    }
    pub fn require_pvt_read(&self, path: &str) -> EResult<()> {
        if self.check_pvt_read(path) {
            Ok(())
        } else {
            Err(Error::access(format!("read access required for: {}", path)))
        }
    }
    pub fn require_pvt_write(&self, path: &str) -> EResult<()> {
        if self.check_pvt_write(path) {
            Ok(())
        } else {
            Err(Error::access(format!(
                "write access required for: {}",
                path
            )))
        }
    }
    pub fn require_rpvt_read(&self, path: &str) -> EResult<()> {
        if self.check_rpvt_read(path) {
            Ok(())
        } else {
            Err(Error::access(format!("read access required for: {}", path)))
        }
    }
    #[inline]
    pub fn contains_acl(&self, acl_id: &str) -> bool {
        self.from.iter().any(|v| v == acl_id)
    }
    #[inline]
    pub fn meta(&self) -> Option<&Value> {
        self.meta.as_ref()
    }
    #[inline]
    pub fn from(&self) -> &[String] {
        &self.from
    }
}

#[cfg(test)]
mod tests {
    use super::{Acl, OIDMask, OIDMaskList, PathMask, PathMaskList};
    use crate::{ItemKind, OID};

    #[test]
    fn test_path_mask() {
        let s = "#";
        let mask: PathMask = s.parse().unwrap();
        assert_eq!(s, mask.to_string());
        assert_eq!(mask.chunks, None);
        let s = "";
        assert_eq!(s.parse::<PathMask>().is_err(), true);
        let s = "data/#";
        let mask: PathMask = s.parse().unwrap();
        assert_eq!(s, mask.to_string());
        assert_eq!(mask.chunks.unwrap(), ["data", "#"]);
        let s = "data/tests/t1";
        let mask: PathMask = s.parse().unwrap();
        assert_eq!(s, mask.to_string());
        assert_eq!(mask.chunks.unwrap(), ["data", "tests", "t1"]);
        let s = "data/tests/*";
        let mask: PathMask = s.parse().unwrap();
        assert_eq!(mask.to_string(), "data/tests/#");
        assert_eq!(mask.chunks.unwrap(), ["data", "tests", "#"]);
        let s = "data/*/t1";
        let mask: PathMask = s.parse().unwrap();
        assert_ne!(s, mask.to_string());
        assert_eq!(mask.chunks.unwrap(), ["data", "#"]);
    }

    #[test]
    fn test_oid_mask() {
        let s = "#";
        let mask: OIDMask = s.parse().unwrap();
        assert_eq!(s, mask.to_string());
        assert_eq!(mask.path.chunks, None);
        assert_eq!(mask.as_path(), "#");
        let s = "";
        assert_eq!(s.parse::<OIDMask>().is_err(), true);
        let s = "sensor:";
        assert_eq!(s.parse::<OIDMask>().is_err(), true);
        let s = "sensor:data/#";
        let mask: OIDMask = s.parse().unwrap();
        assert_eq!(mask.as_path(), "sensor/data/#");
        assert_eq!(s, mask.to_string());
        assert_eq!(mask.kind.unwrap(), ItemKind::Sensor);
        assert_eq!(mask.path.chunks.unwrap(), ["data", "#"]);
        let s = "#:data/#";
        assert_eq!(s.parse::<OIDMask>().is_err(), true);
        let s = "+:data/tests/t1";
        let mask: OIDMask = s.parse().unwrap();
        assert_eq!(mask.as_path(), "+/data/tests/t1");
        assert_eq!(s, mask.to_string());
        assert_eq!(mask.path.chunks.unwrap(), ["data", "tests", "t1"]);
        assert_eq!(mask.kind, None);
        let s = "unit:data/tests/*";
        let mask: OIDMask = s.parse().unwrap();
        assert_eq!(mask.to_string(), "unit:data/tests/#");
        assert_eq!(mask.path.chunks.unwrap(), ["data", "tests", "#"]);
        assert_eq!(mask.kind.unwrap(), ItemKind::Unit);
        let s = "data/*/t1";
        let mask: PathMask = s.parse().unwrap();
        assert_ne!(s, mask.to_string());
        assert_eq!(mask.chunks.unwrap(), ["data", "#"]);
    }

    #[test]
    fn test_path_mask_list() {
        let p =
            PathMaskList::from_str_list(&["test/tests", "+/xxx", "zzz/?/222", "abc", "a/b/#/c"]);
        assert_eq!(p.matches("test"), false);
        assert_eq!(p.matches("test/tests"), true);
        assert_eq!(p.matches("test/tests2"), false);
        assert_eq!(p.matches("aaa/xxx"), true);
        assert_eq!(p.matches("aaa/xxx/123"), false);
        assert_eq!(p.matches("zzz/xxx/222"), true);
        assert_eq!(p.matches("zzz/xxx/222/555"), false);
        assert_eq!(p.matches("zzz/xxx/223"), false);
        assert_eq!(p.matches("abc"), true);
        assert_eq!(p.matches("abd"), false);
        assert_eq!(p.matches("abc/xxx"), true);
        assert_eq!(p.matches("abc/zzz"), false);
        assert_eq!(p.matches("a/b/zzz"), true);
        assert_eq!(p.matches("a/b/zzz/xxx"), true);
        let p = PathMaskList::from_str_list(&["*"]);
        assert_eq!(p.matches("test"), true);
        assert_eq!(p.matches("test/tests"), true);
        assert_eq!(p.matches("test/tests2"), true);
        assert_eq!(p.matches("aaa/xxx"), true);
        assert_eq!(p.matches("aaa/xxx/123"), true);
        assert_eq!(p.matches("zzz/xxx/222"), true);
        assert_eq!(p.matches("zzz/xxx/222/555"), true);
        assert_eq!(p.matches("zzz/xxx/223"), true);
        assert_eq!(p.matches("abc"), true);
        assert_eq!(p.matches("abd"), true);
        assert_eq!(p.matches("abc/xxx"), true);
        assert_eq!(p.matches("abc/zzz"), true);
        assert_eq!(p.matches("a/b/zzz"), true);
        assert_eq!(p.matches("a/b/zzz/xxx"), true);
    }

    #[test]
    fn test_oid_mask_list() {
        let p = OIDMaskList::from_str_list(&[
            "unit:test/tests",
            "sensor:+/xxx",
            "+:zzz/?/222",
            "lvar:abc",
            "+:a/b/#/c",
        ])
        .unwrap();
        assert_eq!(p.matches(&"unit:test".parse().unwrap()), false);
        assert_eq!(p.matches(&"unit:test/tests".parse().unwrap()), true);
        assert_eq!(p.matches(&"sensor:test/tests".parse().unwrap()), false);
        assert_eq!(p.matches(&"unit:test/tests2".parse().unwrap()), false);
        assert_eq!(p.matches(&"sensor:aaa/xxx".parse().unwrap()), true);
        assert_eq!(p.matches(&"lvar:aaa/xxx".parse().unwrap()), false);
        assert_eq!(p.matches(&"unit:zzz/xxx/222".parse().unwrap()), true);
        assert_eq!(p.matches(&"sensor:zzz/xxx/222".parse().unwrap()), true);
        assert_eq!(p.matches(&"sensor:zzz/xxx/222/555".parse().unwrap()), false);
        assert_eq!(p.matches(&"unit:zzz/xxx/223".parse().unwrap()), false);
        assert_eq!(p.matches(&"lvar:abc".parse().unwrap()), true);
        assert_eq!(p.matches(&"unit:abc".parse().unwrap()), false);
        assert_eq!(p.matches(&"lvar:abd".parse().unwrap()), false);
        assert_eq!(p.matches(&"lvar:abc/xxx".parse().unwrap()), false);
        assert_eq!(p.matches(&"sensor:abc/xxx".parse().unwrap()), true);
        assert_eq!(p.matches(&"sensor:abc/zzz".parse().unwrap()), false);
        assert_eq!(p.matches(&"unit:a/b/zzz".parse().unwrap()), true);
        assert_eq!(p.matches(&"unit:a/b/zzz/xxx".parse().unwrap()), true);
        assert_eq!(p.matches(&"unit:a/c/zzz/xxx".parse().unwrap()), false);
        let p = OIDMaskList::from_str_list(&["*"]).unwrap();
        assert_eq!(p.matches(&"unit:test".parse().unwrap()), true);
        assert_eq!(p.matches(&"unit:test/tests".parse().unwrap()), true);
        assert_eq!(p.matches(&"sensor:test/tests".parse().unwrap()), true);
        assert_eq!(p.matches(&"unit:test/tests2".parse().unwrap()), true);
        assert_eq!(p.matches(&"sensor:aaa/xxx".parse().unwrap()), true);
        assert_eq!(p.matches(&"lvar:aaa/xxx".parse().unwrap()), true);
        assert_eq!(p.matches(&"unit:zzz/xxx/222".parse().unwrap()), true);
        assert_eq!(p.matches(&"sensor:zzz/xxx/222".parse().unwrap()), true);
        assert_eq!(p.matches(&"sensor:zzz/xxx/222/555".parse().unwrap()), true);
        assert_eq!(p.matches(&"unit:zzz/xxx/223".parse().unwrap()), true);
        assert_eq!(p.matches(&"lvar:abc".parse().unwrap()), true);
        assert_eq!(p.matches(&"unit:abc".parse().unwrap()), true);
        assert_eq!(p.matches(&"lvar:abd".parse().unwrap()), true);
        assert_eq!(p.matches(&"lvar:abc/xxx".parse().unwrap()), true);
        assert_eq!(p.matches(&"sensor:abc/xxx".parse().unwrap()), true);
        assert_eq!(p.matches(&"sensor:abc/zzz".parse().unwrap()), true);
        assert_eq!(p.matches(&"unit:a/b/zzz".parse().unwrap()), true);
        assert_eq!(p.matches(&"unit:a/b/zzz/xxx".parse().unwrap()), true);
        assert_eq!(p.matches(&"unit:a/c/zzz/xxx".parse().unwrap()), true);

        let p = OIDMaskList::from_str_list(&["sensor:content/#"]).unwrap();
        assert_eq!(p.matches(&"sensor:content/data".parse().unwrap()), true);
        let p = OIDMaskList::from_str_list(&["sensor:+"]).unwrap();
        assert_ne!(p.matches(&"sensor:content/data".parse().unwrap()), true);
    }

    #[test]
    fn test_oid_wildcard_mask() {
        let mask: OIDMask = "sensor:tests/#".parse().unwrap();
        let oid_mask: OID = mask.to_wildcard_oid().unwrap();
        assert!(oid_mask.is_wildcard());
        assert_eq!(oid_mask.to_wildcard_str("%"), "sensor:tests/%");
        let mask: OIDMask = "sensor:#".parse().unwrap();
        let oid_mask: OID = mask.to_wildcard_oid().unwrap();
        assert!(oid_mask.is_wildcard());
        assert_eq!(oid_mask.to_wildcard_str("%"), "sensor:%");
        let mask: OIDMask = "sensor:+/#".parse().unwrap();
        assert!(mask.to_wildcard_oid().is_err());
    }

    #[test]
    fn test_rpvt_acl() {
        let p_allow = PathMaskList::from_str_list(&["node1/res", "node2/res/#"]);
        let p_deny = PathMaskList::from_str_list(&["node2/res/secret"]);
        let mut acl: Acl = serde_json::from_str(
            r#"{
        "id": "test",
        "from": ["test"]
        }"#,
        )
        .unwrap();
        acl.read.rpvt = p_allow;
        acl.deny_read.rpvt = p_deny;
        for pfx in &["", "http://", "https://"] {
            assert_eq!(acl.check_rpvt_read(&format!("node1/{pfx}res")), true);
            assert_eq!(acl.check_rpvt_read(&format!("node2/{pfx}res")), false);
            assert_eq!(acl.check_rpvt_read(&format!("node2/{pfx}res/res1")), true);
            assert_eq!(
                acl.check_rpvt_read(&format!("node2/{pfx}res/secret")),
                false
            );
            assert_eq!(acl.check_rpvt_read(&format!("node3/{pfx}res")), false);
        }
    }
}
