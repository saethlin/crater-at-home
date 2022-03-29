use std::fmt;

use serde::de::Visitor;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Crate {
    pub name: String,
    pub recent_downloads: Option<u64>,
    pub version: String,
    pub status: Status,
    #[serde(default)]
    /// Time that the run took, in seconds
    pub time: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Status {
    Unknown,
    Passing,
    Error(String),
    UB {
        #[serde(deserialize_with = "cause_version_remap")]
        cause: Vec<Cause>,
        status: String,
    },
}

fn cause_version_remap<'de, D>(deserializer: D) -> Result<Vec<Cause>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct StringOrStruct;

    impl<'de> Visitor<'de> for StringOrStruct {
        type Value = Vec<Cause>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }

        fn visit_str<E>(self, value: &str) -> Result<Vec<Cause>, E>
        where
            E: serde::de::Error,
        {
            let mut causes = Vec::new();
            for cause in value.split(',') {
                let mut splits = cause.split_terminator('(');
                let kind = splits.next().unwrap().trim().to_string();
                let source_crate = splits
                    .next()
                    .map(|s| s.trim_end_matches(')').trim().to_string());
                causes.push(Cause { kind, source_crate })
            }
            Ok(causes)
        }

        fn visit_seq<M>(self, map: M) -> Result<Vec<Cause>, M::Error>
        where
            M: serde::de::SeqAccess<'de>,
        {
            serde::Deserialize::deserialize(serde::de::value::SeqAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(StringOrStruct)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Ord, Eq, PartialEq, PartialOrd)]
pub struct Cause {
    pub kind: String,
    pub source_crate: Option<String>,
}
